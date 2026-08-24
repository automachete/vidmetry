[CmdletBinding()]
param(
    [string]$PackagePath,
    [switch]$LiveInstall
)

$ErrorActionPreference = 'Stop'
$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$ownedRegistryKeys = @(
    'Software\Classes\Vidmetry.Video',
    'Software\Classes\Applications\vidmetry.exe',
    'Software\Classes\Directory\shell\Vidmetry',
    'Software\Vidmetry',
    'Software\Microsoft\Windows\CurrentVersion\Uninstall\Vidmetry'
)
$shortcutPaths = @(
    (Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::DesktopDirectory)) 'Vidmetry.lnk'),
    (Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::Programs)) 'Vidmetry.lnk')
)

function Open-CurrentUserKey([string]$Path, [bool]$Writable = $false) {
    [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($Path, $Writable)
}

function Get-PeMachine([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    $reader = [System.IO.BinaryReader]::new($stream)
    try {
        if ($reader.ReadUInt16() -ne 0x5A4D) {
            throw "$Path is not a PE file."
        }
        $stream.Position = 0x3C
        $peOffset = $reader.ReadInt32()
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) {
            throw "$Path has an invalid PE signature."
        }
        $reader.ReadUInt16()
    } finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

function Assert-BundleMarker([string]$Path, [string]$Expected) {
    $binaryText = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($Path))
    if (-not $binaryText.Contains($Expected, [StringComparison]::Ordinal)) {
        throw "$Path does not contain the expected Tauri bundle marker $Expected."
    }
}

function Assert-RegistryValue([string]$Path, [string]$Name, $Expected) {
    $key = Open-CurrentUserKey $Path
    if ($null -eq $key) {
        throw "Required registry key is missing: HKCU\$Path"
    }
    try {
        if ($Name -cnotin @($key.GetValueNames())) {
            throw "Required registry value is missing: HKCU\$Path [$Name]"
        }
        $actual = $key.GetValue(
            $Name,
            $null,
            [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
        )
        if ([string]$actual -cne [string]$Expected) {
            throw "Unexpected registry value at HKCU\$Path [$Name]: '$actual'"
        }
    } finally {
        $key.Dispose()
    }
}

function Assert-RegistryKeyMissing([string]$Path) {
    $key = Open-CurrentUserKey $Path
    if ($null -ne $key) {
        $key.Dispose()
        throw "Registry key must be absent: HKCU\$Path"
    }
}

function Assert-RegistryValueMissing([string]$Path, [string]$Name) {
    $key = Open-CurrentUserKey $Path
    if ($null -eq $key) {
        return
    }
    try {
        if ($Name -cin @($key.GetValueNames())) {
            throw "Registry value must be absent: HKCU\$Path [$Name]"
        }
    } finally {
        $key.Dispose()
    }
}

function Invoke-Nsis([string]$Installer, [string[]]$Arguments) {
    $process = Start-Process -FilePath $Installer -ArgumentList $Arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "NSIS process failed with exit code $($process.ExitCode): $Installer"
    }
}

function Remove-TestRegistry {
    foreach ($path in $ownedRegistryKeys) {
        try {
            [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($path, $false)
        } catch [System.ArgumentException] {
        }
    }
    $registeredApplications = Open-CurrentUserKey 'Software\RegisteredApplications' $true
    if ($null -ne $registeredApplications) {
        try {
            $registeredApplications.DeleteValue('Vidmetry', $false)
        } finally {
            $registeredApplications.Dispose()
        }
    }
    foreach ($extension in $script:extensions) {
        $path = "Software\Classes\.$extension\OpenWithProgids"
        $key = Open-CurrentUserKey $path $true
        if ($null -ne $key) {
            try {
                $key.DeleteValue('Vidmetry.Video', $false)
            } finally {
                $key.Dispose()
            }
        }
    }
}

$version = (Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json).version
if ([string]::IsNullOrWhiteSpace($PackagePath)) {
    $PackagePath = Join-Path $projectRoot 'src-tauri\target\release\bundle\nsis\Vidmetry_x64-setup.exe'
} else {
    $PackagePath = [System.IO.Path]::GetFullPath($PackagePath)
}
if (-not (Test-Path -LiteralPath $PackagePath -PathType Leaf)) {
    throw "NSIS package not found: $PackagePath"
}
if ((Split-Path -Leaf $PackagePath) -cne 'Vidmetry_x64-setup.exe') {
    throw "Unexpected NSIS package name: $(Split-Path -Leaf $PackagePath)"
}
$versionInfo = (Get-Item -LiteralPath $PackagePath).VersionInfo
if ($versionInfo.ProductName -cne 'Vidmetry' -or $versionInfo.ProductVersion -cne $version -or
    $versionInfo.FileVersion -cne $version) {
    throw "Unexpected NSIS embedded product/version metadata: $($versionInfo.ProductName) $($versionInfo.ProductVersion) $($versionInfo.FileVersion)"
}

$selectionSource = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri\src\selection.rs') -Raw
$extensionBlock = [regex]::Match(
    $selectionSource,
    'const\s+VIDEO_EXTENSIONS\s*:\s*&\[&str\]\s*=\s*&\[(?<extensions>[\s\S]*?)\];'
)
$script:extensions = @([regex]::Matches($extensionBlock.Groups['extensions'].Value, '"(?<extension>[a-z0-9]+)"') |
    ForEach-Object { $_.Groups['extension'].Value })
if ($script:extensions.Count -eq 0) {
    throw 'The backend video-extension contract is empty.'
}

if (-not $LiveInstall) {
    Write-Output "NSIS package and filename contract are valid: $PackagePath"
    exit 0
}

foreach ($path in $ownedRegistryKeys) {
    Assert-RegistryKeyMissing $path
}
foreach ($shortcutPath in $shortcutPaths) {
    if (Test-Path -LiteralPath $shortcutPath) {
        throw "A pre-existing Vidmetry shortcut prevents an isolated live test: $shortcutPath"
    }
}
$registeredApplications = Open-CurrentUserKey 'Software\RegisteredApplications'
if ($null -ne $registeredApplications) {
    try {
        if ('Vidmetry' -cin @($registeredApplications.GetValueNames())) {
            throw 'A pre-existing Vidmetry RegisteredApplications value prevents an isolated live test.'
        }
    } finally {
        $registeredApplications.Dispose()
    }
}
foreach ($extension in $script:extensions) {
    $key = Open-CurrentUserKey "Software\Classes\.$extension\OpenWithProgids"
    if ($null -ne $key) {
        try {
            if ('Vidmetry.Video' -cin @($key.GetValueNames())) {
                throw "A pre-existing .$extension Vidmetry registration prevents an isolated live test."
            }
        } finally {
            $key.Dispose()
        }
    }
}

$operationRoot = Join-Path ([System.IO.Path]::GetTempPath()) "vidmetry-nsis-$([guid]::NewGuid())"
$installDirectory = Join-Path $operationRoot 'app'
$migrationInstallDirectory = Join-Path $operationRoot 'migration-app'
$installDirectories = @($installDirectory, $migrationInstallDirectory)
$selectionDirectory = Join-Path $operationRoot 'selection'
$launchedProcess = $null
$uninstaller = $null
New-Item -ItemType Directory -Path $selectionDirectory | Out-Null
try {
    Invoke-Nsis $PackagePath @('/S', "/D=$installDirectory")

    foreach ($required in @(
        'vidmetry.exe',
        'ffmpeg.exe',
        'ffprobe.exe',
        'LICENSE',
        'THIRD_PARTY_NOTICES.md',
        'FFmpeg\FFMPEG_LICENSE.txt',
        'FFmpeg\FFMPEG_BUILD_INFO.txt',
        'FFmpeg\FFMPEG_CORRESPONDING_SOURCE.txt',
        'ThirdPartyLicenses\RUST_THIRD_PARTY_LICENSES.html',
        'ThirdPartyLicenses\JAVASCRIPT_THIRD_PARTY_LICENSES.txt',
        'LicenseSources\MPL-2.0\SOURCE_INDEX.txt',
        'shortcut-icon-achromatic-v2.ico'
    )) {
        $requiredPath = Join-Path $installDirectory $required
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "The installed NSIS payload is missing $required."
        }
    }
    if ((Get-PeMachine (Join-Path $installDirectory 'vidmetry.exe')) -ne 0x8664) {
        throw 'The NSIS application must be an x64 PE file.'
    }
    Assert-BundleMarker (Join-Path $installDirectory 'vidmetry.exe') '__TAURI_BUNDLE_TYPE_VAR_NSS'
    $forbiddenFiles = @(Get-ChildItem -LiteralPath $installDirectory -File -Recurse |
        Where-Object { $_.Extension -in @('.pdb', '.lib', '.exp', '.iobj', '.ipdb') })
    if ($forbiddenFiles.Count -ne 0) {
        throw "The NSIS package contains build-only files: $($forbiddenFiles.Name -join ', ')"
    }

    $ffmpegManifest = Get-Content -LiteralPath (Join-Path $projectRoot 'scripts\ffmpeg-sidecars.json') -Raw | ConvertFrom-Json
    $target = $ffmpegManifest.targets.'x86_64-pc-windows-msvc'
    if ((Get-FileHash -LiteralPath (Join-Path $installDirectory 'ffmpeg.exe') -Algorithm SHA256).Hash.ToLowerInvariant() -cne $target.ffmpeg.sha256 -or
        (Get-FileHash -LiteralPath (Join-Path $installDirectory 'ffprobe.exe') -Algorithm SHA256).Hash.ToLowerInvariant() -cne $target.ffprobe.sha256) {
        throw 'The installed NSIS sidecar hashes do not match the locked runtime manifest.'
    }

    foreach ($extension in $script:extensions) {
        Assert-RegistryValue "Software\Classes\.$extension\OpenWithProgids" 'Vidmetry.Video' ''
        Assert-RegistryValue 'Software\Classes\Applications\vidmetry.exe\SupportedTypes' ".$extension" ''
        Assert-RegistryValue 'Software\Vidmetry\Capabilities\FileAssociations' ".$extension" 'Vidmetry.Video'
    }
    $expectedCommand = "`"$installDirectory\vidmetry.exe`" `"%1`""
    Assert-RegistryValue 'Software\Classes\Vidmetry.Video\shell\open\command' '' $expectedCommand
    Assert-RegistryValue 'Software\Classes\Directory\shell\Vidmetry' '' 'Open with Vidmetry'
    Assert-RegistryValue 'Software\Classes\Directory\shell\Vidmetry' 'MultiSelectModel' 'Single'
    Assert-RegistryValue 'Software\Classes\Directory\shell\Vidmetry\command' '' $expectedCommand
    Assert-RegistryValue 'Software\Vidmetry' 'NsisExplorerIntegrationEnabled' 1

    $launchedProcess = Start-Process `
        -FilePath (Join-Path $installDirectory 'vidmetry.exe') `
        -ArgumentList "`"$selectionDirectory`"" `
        -PassThru
    if (-not $launchedProcess.WaitForInputIdle(10000)) {
        throw 'The NSIS-installed Vidmetry process did not reach an interactive state.'
    }
    if ($launchedProcess.ProcessName -cne 'vidmetry') {
        throw "The NSIS installation launched an unexpected process: $($launchedProcess.ProcessName)"
    }
    Stop-Process -Id $launchedProcess.Id -Force
    $null = $launchedProcess.WaitForExit(10000)
    $launchedProcess = $null

    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree(
        'Software\Classes\Directory\shell\Vidmetry',
        $false
    )
    $stateKey = Open-CurrentUserKey 'Software\Vidmetry' $true
    try {
        $stateKey.SetValue('NsisExplorerIntegrationEnabled', 0, [Microsoft.Win32.RegistryValueKind]::DWord)
    } finally {
        $stateKey.Dispose()
    }
    Invoke-Nsis $PackagePath @('/S', "/D=$installDirectory")
    Assert-RegistryKeyMissing 'Software\Classes\Directory\shell\Vidmetry'
    Assert-RegistryValue 'Software\Classes\.mp4\OpenWithProgids' 'Vidmetry.Video' ''
    Assert-RegistryValue 'Software\Vidmetry' 'NsisExplorerIntegrationEnabled' 0

    $uninstallers = @(Get-ChildItem -LiteralPath $installDirectory -Filter 'uninstall*.exe' -File)
    if ($uninstallers.Count -ne 1) {
        throw "Expected one NSIS uninstaller but found $($uninstallers.Count)."
    }
    $uninstaller = $uninstallers[0].FullName
    Invoke-Nsis $uninstaller @('/S')
    $uninstaller = $null

    foreach ($path in $ownedRegistryKeys) {
        Assert-RegistryKeyMissing $path
    }
    foreach ($extension in $script:extensions) {
        $key = Open-CurrentUserKey "Software\Classes\.$extension\OpenWithProgids"
        if ($null -ne $key) {
            try {
                if ('Vidmetry.Video' -cin @($key.GetValueNames())) {
                    throw "The NSIS uninstaller left the .$extension Vidmetry registration behind."
                }
            } finally {
                $key.Dispose()
            }
        }
    }

    $legacyStateKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Software\Vidmetry')
    try {
        $legacyStateKey.SetValue('ExplorerIntegrationEnabled', 0, [Microsoft.Win32.RegistryValueKind]::DWord)
    } finally {
        $legacyStateKey.Dispose()
    }
    Invoke-Nsis $PackagePath @('/S', "/D=$migrationInstallDirectory")
    Assert-RegistryKeyMissing 'Software\Classes\Directory\shell\Vidmetry'
    Assert-RegistryValue 'Software\Classes\.mp4\OpenWithProgids' 'Vidmetry.Video' ''
    Assert-RegistryValue 'Software\Vidmetry' 'NsisExplorerIntegrationEnabled' 0
    Assert-RegistryValue 'Software\Vidmetry' 'ExplorerIntegrationEnabled' 0

    $uninstallers = @(Get-ChildItem -LiteralPath $migrationInstallDirectory -Filter 'uninstall*.exe' -File)
    if ($uninstallers.Count -ne 1) {
        throw "Expected one migration-test NSIS uninstaller but found $($uninstallers.Count)."
    }
    $uninstaller = $uninstallers[0].FullName
    Invoke-Nsis $uninstaller @('/S')
    $uninstaller = $null
    foreach ($path in $ownedRegistryKeys | Where-Object { $_ -cne 'Software\Vidmetry' }) {
        Assert-RegistryKeyMissing $path
    }
    Assert-RegistryValue 'Software\Vidmetry' 'ExplorerIntegrationEnabled' 0
    Assert-RegistryValueMissing 'Software\Vidmetry' 'NsisExplorerIntegrationEnabled'
    Assert-RegistryValueMissing 'Software\Classes\.mp4\OpenWithProgids' 'Vidmetry.Video'

    Write-Output "NSIS live install, $($script:extensions.Count) video associations, folder-only updates, legacy-state migration, app launch, and uninstall passed."
} finally {
    if ($null -ne $launchedProcess -and -not $launchedProcess.HasExited) {
        Stop-Process -Id $launchedProcess.Id -Force
        $null = $launchedProcess.WaitForExit(10000)
    }
    $cleanupUninstallers = @()
    if ($null -ne $uninstaller) {
        $cleanupUninstallers += $uninstaller
    }
    foreach ($directory in $installDirectories) {
        if (Test-Path -LiteralPath $directory -PathType Container) {
            $cleanupUninstallers += @(Get-ChildItem -LiteralPath $directory -Filter 'uninstall*.exe' -File |
                ForEach-Object FullName)
        }
    }
    foreach ($cleanupUninstaller in $cleanupUninstallers | Select-Object -Unique) {
        if (-not (Test-Path -LiteralPath $cleanupUninstaller -PathType Leaf)) {
            continue
        }
        try {
            Invoke-Nsis $cleanupUninstaller @('/S')
        } catch {
        }
    }
    Remove-TestRegistry
    foreach ($shortcutPath in $shortcutPaths) {
        if (Test-Path -LiteralPath $shortcutPath -PathType Leaf) {
            Remove-Item -LiteralPath $shortcutPath -Force
        }
    }
    if (Test-Path -LiteralPath $operationRoot) {
        Remove-Item -LiteralPath $operationRoot -Recurse -Force
    }
}
