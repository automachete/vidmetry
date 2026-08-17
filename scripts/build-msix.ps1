[CmdletBinding()]
param(
    [ValidatePattern('^[A-Za-z0-9.-]{3,50}$')]
    [string]$IdentityName = 'Vidmetry.Dev',

    [ValidateNotNullOrEmpty()]
    [string]$Publisher = 'CN=Vidmetry Development',

    [ValidateNotNullOrEmpty()]
    [string]$PublisherDisplayName = 'Vidmetry',

    [string]$CertificateThumbprint,

    [switch]$SkipAppBuild,

    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$releaseDirectory = Join-Path $projectRoot 'src-tauri\target\release'
$bundleDirectory = Join-Path $releaseDirectory 'bundle\msix'
$layoutDirectory = Join-Path $bundleDirectory 'layout'
$manifestTemplatePath = Join-Path $PSScriptRoot 'msix\AppxManifest.xml.template'

function Assert-ChildPath([string]$Path, [string]$Parent, [string]$Label) {
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    $resolvedParent = [System.IO.Path]::GetFullPath($Parent).TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
        [System.IO.Path]::DirectorySeparatorChar
    if (-not $resolvedPath.StartsWith($resolvedParent, [StringComparison]::OrdinalIgnoreCase)) {
        throw "$Label must remain under $resolvedParent"
    }
}

function Get-WindowsSdkTool([string]$Name) {
    $sdkBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    if (-not (Test-Path -LiteralPath $sdkBin -PathType Container)) {
        throw 'Windows SDK 10 is required to build an MSIX package.'
    }

    $tools = @(Get-ChildItem -LiteralPath $sdkBin -Filter $Name -File -Recurse |
        Where-Object { $_.Directory.Name -ceq 'x64' } |
        Sort-Object { [Version]($_.Directory.Parent.Name -replace '[^0-9.]', '') } -Descending)
    if ($tools.Count -eq 0) {
        throw "$Name was not found in Windows SDK 10."
    }
    $tools[0].FullName
}

function Get-VisualStudioBuildTools {
    $vsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vsWhere -PathType Leaf)) {
        throw 'Visual Studio Installer is required to locate the C++ build tools.'
    }
    $installationPath = (& $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath |
        Select-Object -First 1).Trim()
    if ([string]::IsNullOrWhiteSpace($installationPath)) {
        throw 'Visual Studio C++ x64 build tools are required for the MSIX Explorer command.'
    }
    $preferred = Join-Path $installationPath 'MSBuild\Current\Bin\amd64\MSBuild.exe'
    if (Test-Path -LiteralPath $preferred -PathType Leaf) {
        $msBuild = $preferred
    } else {
        $fallback = @(Get-ChildItem -LiteralPath (Join-Path $installationPath 'MSBuild') -Filter 'MSBuild.exe' -File -Recurse |
            Select-Object -First 1)
        if ($fallback.Count -ne 1) {
            throw 'MSBuild.exe was not found in the selected Visual Studio installation.'
        }
        $msBuild = $fallback[0].FullName
    }

    $vcTargetsRoot = Join-Path $installationPath 'MSBuild\Microsoft\VC'
    $vcTargets = @(Get-ChildItem -LiteralPath $vcTargetsRoot -Directory |
        Where-Object { $_.Name -match '^v(?<version>\d+)$' } |
        Sort-Object { [int]($_.Name.Substring(1)) } -Descending)
    $platformToolset = $null
    foreach ($vcTarget in $vcTargets) {
        $toolsetDirectory = Join-Path $vcTarget.FullName 'Platforms\x64\PlatformToolsets'
        if (-not (Test-Path -LiteralPath $toolsetDirectory -PathType Container)) {
            continue
        }
        $platformToolset = @(Get-ChildItem -LiteralPath $toolsetDirectory -Directory |
            Where-Object { $_.Name -match '^v(?<version>\d+)$' } |
            Sort-Object { [int]($_.Name.Substring(1)) } -Descending |
            Select-Object -First 1)
        if ($platformToolset.Count -eq 1) {
            break
        }
    }
    if ($null -eq $platformToolset -or $platformToolset.Count -ne 1) {
        throw 'An x64 Visual C++ platform toolset was not found.'
    }

    [pscustomobject]@{
        MSBuild = $msBuild
        PlatformToolset = $platformToolset[0].Name
    }
}

function ConvertTo-MsixVersion([string]$Version) {
    if ($Version -notmatch '^(?<major>\d+)\.(?<minor>\d+)\.(?<patch>\d+)$') {
        throw "MSIX releases require a numeric major.minor.patch version, got '$Version'."
    }
    $parts = @([int]$Matches.major, [int]$Matches.minor, [int]$Matches.patch, 0)
    if (@($parts | Where-Object { $_ -lt 0 -or $_ -gt 65535 }).Count -ne 0) {
        throw "MSIX version components must be between 0 and 65535: '$Version'."
    }
    $parts -join '.'
}

function ConvertTo-XmlAttribute([string]$Value) {
    [System.Security.SecurityElement]::Escape($Value)
}

function Copy-RequiredFile([string]$Source, [string]$Destination) {
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        throw "Required MSIX input is missing: $Source"
    }
    Copy-Item -LiteralPath $Source -Destination $Destination
}

function Copy-RequiredDirectory([string]$Source, [string]$Destination) {
    if (-not (Test-Path -LiteralPath $Source -PathType Container)) {
        throw "Required MSIX input is missing: $Source"
    }
    Copy-Item -LiteralPath $Source -Destination $Destination -Recurse
}

$package = Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json
$cargoManifest = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri\Cargo.toml') -Raw
$tauriConfig = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$cargoVersionMatch = [regex]::Match($cargoManifest, '(?m)^version\s*=\s*"(?<version>[^"]+)"\s*$')
if (-not $cargoVersionMatch.Success) {
    throw 'Unable to read the Rust package version.'
}
$versions = @($package.version, $cargoVersionMatch.Groups['version'].Value, $tauriConfig.version)
if (@($versions | Select-Object -Unique).Count -ne 1) {
    throw "Application versions do not match: $($versions -join ', ')."
}
$msixVersion = ConvertTo-MsixVersion $package.version

if (-not $SkipAppBuild) {
    & npm run tauri build -- --no-bundle
    if ($LASTEXITCODE -ne 0) {
        throw 'Tauri release build failed.'
    }
}

$explorerCommandProject = Join-Path $projectRoot 'src-tauri\windows\msix-explorer-command\ExplorerCommand.vcxproj'
$explorerCommandBuildDirectory = Join-Path $releaseDirectory 'msix-explorer-command'
$explorerCommandObjectDirectory = Join-Path $explorerCommandBuildDirectory 'obj'
New-Item -ItemType Directory -Path $explorerCommandBuildDirectory -Force | Out-Null
$buildTools = Get-VisualStudioBuildTools
& $buildTools.MSBuild $explorerCommandProject /nologo /m /t:Rebuild "/p:Configuration=Release" "/p:Platform=x64" "/p:PlatformToolset=$($buildTools.PlatformToolset)" "/p:OutDir=$explorerCommandBuildDirectory\" "/p:IntDir=$explorerCommandObjectDirectory\"
if ($LASTEXITCODE -ne 0) {
    throw 'The MSIX Explorer command build failed.'
}
$explorerCommandDll = Join-Path $explorerCommandBuildDirectory 'VidmetryExplorerCommand.dll'
if (-not (Test-Path -LiteralPath $explorerCommandDll -PathType Leaf)) {
    throw 'The MSIX Explorer command DLL was not generated.'
}

Assert-ChildPath $layoutDirectory $projectRoot 'MSIX layout directory'
if (Test-Path -LiteralPath $layoutDirectory) {
    Remove-Item -LiteralPath $layoutDirectory -Recurse -Force
}
New-Item -ItemType Directory -Path $layoutDirectory | Out-Null

foreach ($file in @(
    'vidmetry.exe',
    'ffmpeg.exe',
    'ffprobe.exe',
    'LICENSE',
    'THIRD_PARTY_NOTICES.md'
)) {
    Copy-RequiredFile (Join-Path $releaseDirectory $file) $layoutDirectory
}
foreach ($directory in @('FFmpeg', 'LicenseSources', 'ThirdPartyLicenses')) {
    Copy-RequiredDirectory (Join-Path $releaseDirectory $directory) $layoutDirectory
}
Copy-RequiredFile $explorerCommandDll $layoutDirectory

$assetsDirectory = Join-Path $layoutDirectory 'Assets'
New-Item -ItemType Directory -Path $assetsDirectory | Out-Null
$iconDirectory = Join-Path $projectRoot 'src-tauri\icons'
foreach ($asset in @(
    'Square44x44Logo.png',
    'Square71x71Logo.png',
    'Square150x150Logo.png',
    'Square310x310Logo.png',
    'StoreLogo.png'
)) {
    Copy-RequiredFile (Join-Path $iconDirectory $asset) $assetsDirectory
}

$manifest = Get-Content -LiteralPath $manifestTemplatePath -Raw
$selectionSource = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri\src\selection.rs') -Raw
$extensionBlock = [regex]::Match(
    $selectionSource,
    'const\s+VIDEO_EXTENSIONS\s*:\s*&\[&str\]\s*=\s*&\[(?<extensions>[\s\S]*?)\];'
)
if (-not $extensionBlock.Success) {
    throw 'Unable to read the backend video-extension contract.'
}
$videoExtensions = @([regex]::Matches($extensionBlock.Groups['extensions'].Value, '"(?<extension>[a-z0-9]+)"') |
    ForEach-Object { $_.Groups['extension'].Value })
if ($videoExtensions.Count -eq 0) {
    throw 'The backend video-extension contract is empty.'
}
$fileTypes = ($videoExtensions | ForEach-Object { "              <uap:FileType>.$_</uap:FileType>" }) -join "`r`n"
$replacements = [ordered]@{
    '@@IDENTITY_NAME@@' = ConvertTo-XmlAttribute $IdentityName
    '@@PUBLISHER@@' = ConvertTo-XmlAttribute $Publisher
    '@@PUBLISHER_DISPLAY_NAME@@' = ConvertTo-XmlAttribute $PublisherDisplayName
    '@@VERSION@@' = $msixVersion
    '@@FILE_TYPES@@' = $fileTypes
}
foreach ($replacement in $replacements.GetEnumerator()) {
    $manifest = $manifest.Replace($replacement.Key, $replacement.Value, [StringComparison]::Ordinal)
}
$manifestPath = Join-Path $layoutDirectory 'AppxManifest.xml'
[System.IO.File]::WriteAllText($manifestPath, $manifest, [System.Text.UTF8Encoding]::new($false))

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = $bundleDirectory
} else {
    $OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$packagePath = Join-Path $OutputDirectory "Vidmetry_$($msixVersion)_x64.msix"
if (Test-Path -LiteralPath $packagePath) {
    Remove-Item -LiteralPath $packagePath -Force
}

$makeAppx = Get-WindowsSdkTool 'makeappx.exe'
& $makeAppx pack /o /d $layoutDirectory /p $packagePath
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $packagePath -PathType Leaf)) {
    throw 'MakeAppx failed to create the MSIX package.'
}

if (-not [string]::IsNullOrWhiteSpace($CertificateThumbprint)) {
    if ($CertificateThumbprint -notmatch '^[0-9A-Fa-f]{40}$') {
        throw 'CertificateThumbprint must contain exactly 40 hexadecimal characters.'
    }
    $signTool = Get-WindowsSdkTool 'signtool.exe'
    & $signTool sign /fd SHA256 /sha1 $CertificateThumbprint $packagePath
    if ($LASTEXITCODE -ne 0) {
        throw 'SignTool failed to sign the MSIX package.'
    }
}

$packageHash = (Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Output "MSIX package: $packagePath"
Write-Output "SHA-256: $packageHash"
