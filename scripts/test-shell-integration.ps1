$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$selectionPath = Join-Path $projectRoot 'src-tauri\src\selection.rs'
$configPath = Join-Path $projectRoot 'src-tauri\tauri.conf.json'
$nsisHookPath = Join-Path $projectRoot 'src-tauri\windows\hooks.nsh'
$msixManifestPath = Join-Path $projectRoot 'scripts\msix\AppxManifest.xml.template'
$msixBuildPath = Join-Path $projectRoot 'scripts\build-msix.ps1'
$msixCommandPath = Join-Path $projectRoot 'src-tauri\windows\msix-explorer-command\ExplorerCommand.cpp'
$msixProjectPath = Join-Path $projectRoot 'src-tauri\windows\msix-explorer-command\ExplorerCommand.vcxproj'

$selection = Get-Content -LiteralPath $selectionPath -Raw
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$nsisHook = Get-Content -LiteralPath $nsisHookPath -Raw
$msixManifest = Get-Content -LiteralPath $msixManifestPath -Raw
$msixBuild = Get-Content -LiteralPath $msixBuildPath -Raw
$msixCommand = Get-Content -LiteralPath $msixCommandPath -Raw
$msixProject = Get-Content -LiteralPath $msixProjectPath -Raw

$extensionBlock = [regex]::Match(
    $selection,
    'const\s+VIDEO_EXTENSIONS\s*:\s*&\[&str\]\s*=\s*&\[(?<extensions>[\s\S]*?)\];'
)
if (-not $extensionBlock.Success) {
    throw 'Unable to read the backend video-extension contract.'
}
$extensions = [regex]::Matches($extensionBlock.Groups['extensions'].Value, '"(?<extension>[a-z0-9]+)"') |
    ForEach-Object { $_.Groups['extension'].Value }
if ($extensions.Count -eq 0) {
    throw 'The backend video-extension contract is empty.'
}

foreach ($extension in $extensions) {
    $register = "!insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION `"$extension`""
    $unregister = "!insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION `"$extension`""
    if (-not $nsisHook.Contains($register) -or -not $nsisHook.Contains($unregister)) {
        throw "The NSIS video Open with integration does not cover .$extension."
    }
}

$postInstall = [regex]::Match(
    $nsisHook,
    '(?ms)!macro NSIS_HOOK_POSTINSTALL\r?\n(?<Body>.*?)!macroend'
)
$directoryUnregister = [regex]::Match(
    $nsisHook,
    '(?ms)!macro VIDMETRY_UNREGISTER_DIRECTORY_INTEGRATION\r?\n(?<Body>.*?)!macroend'
)
if (-not $postInstall.Success -or -not $directoryUnregister.Success) {
    throw 'The NSIS Explorer integration macros could not be parsed.'
}
$postInstallBody = $postInstall.Groups['Body'].Value
$nsisStateRead = $postInstallBody.IndexOf('"NsisExplorerIntegrationEnabled"', [StringComparison]::Ordinal)
$legacyStateRead = $postInstallBody.IndexOf('"ExplorerIntegrationEnabled"', [StringComparison]::Ordinal)
if ($postInstallBody.IndexOf('VIDMETRY_REGISTER_VIDEO_INTEGRATION', [StringComparison]::Ordinal) -lt 0 -or
    $postInstallBody.IndexOf('VIDMETRY_REGISTER_VIDEO_INTEGRATION', [StringComparison]::Ordinal) -gt
        $postInstallBody.IndexOf('ReadRegDWORD', [StringComparison]::Ordinal) -or
    $nsisStateRead -lt 0 -or $legacyStateRead -le $nsisStateRead -or
    -not $postInstallBody.Contains('VIDMETRY_REGISTER_DIRECTORY_INTEGRATION', [StringComparison]::Ordinal) -or
    -not $postInstallBody.Contains('VIDMETRY_UNREGISTER_DIRECTORY_INTEGRATION', [StringComparison]::Ordinal)) {
    throw 'NSIS must always register video Open with entries and toggle only the directory command.'
}
$directoryUnregisterBody = $directoryUnregister.Groups['Body'].Value
foreach ($forbidden in @('OpenWithProgids', 'Applications\vidmetry.exe', 'Capabilities')) {
    if ($directoryUnregisterBody.Contains($forbidden, [StringComparison]::Ordinal)) {
        throw "Disabling the NSIS directory command must not remove video registration '$forbidden'."
    }
}
foreach ($required in @(
    'Software\Classes\Vidmetry.Video',
    'Software\Classes\Applications\vidmetry.exe',
    'Software\Classes\Directory\shell\Vidmetry',
    'Open with Vidmetry',
    'MultiSelectModel',
    'NsisExplorerIntegrationEnabled',
    'ExplorerIntegrationEnabled',
    'NSIS_HOOK_POSTUNINSTALL',
    'SHChangeNotify'
)) {
    if (-not $nsisHook.Contains($required, [StringComparison]::Ordinal)) {
        throw "The NSIS Explorer integration is missing '$required'."
    }
}

$commandClsid = '7CD16804-1388-4150-991B-A977AEA22567'
$requiredMsixManifestContent = @(
    'Category="windows.fileTypeAssociation"',
    'desktop2:AllowSilentDefaultTakeOver="true"',
    '@@FILE_TYPES@@',
    'Category="windows.comServer"',
    'Path="VidmetryExplorerCommand.dll"',
    'Category="windows.fileExplorerContextMenus"',
    '<desktop5:ItemType Type="Directory">',
    $commandClsid
)
foreach ($required in $requiredMsixManifestContent) {
    if (-not $msixManifest.Contains($required)) {
        throw "The MSIX Explorer integration is missing '$required'."
    }
}

$requiredMsixBuildContent = @(
    'VIDEO_EXTENSIONS',
    '@@FILE_TYPES@@',
    'VidmetryExplorerCommand.dll',
    'ExplorerCommand.vcxproj'
)
foreach ($required in $requiredMsixBuildContent) {
    if (-not $msixBuild.Contains($required)) {
        throw "The MSIX build does not preserve '$required'."
    }
}

$requiredMsixCommandContent = @(
    'IExplorerCommand',
    'GetCurrentPackageFamilyName',
    'ActivateApplication',
    'SIGDN_FILESYSPATH',
    'ExplorerIntegrationEnabled',
    'ECS_HIDDEN',
    'DllGetClassObject'
)
foreach ($required in $requiredMsixCommandContent) {
    if (-not $msixCommand.Contains($required)) {
        throw "The MSIX Explorer command is missing '$required'."
    }
}
if (-not $msixCommand.Contains('0x7cd16804') -or -not $msixCommand.Contains('0x1388')) {
    throw 'The MSIX Explorer command CLSID does not match the package manifest.'
}
foreach ($required in @('<Platform>x64</Platform>', '<RuntimeLibrary>MultiThreaded</RuntimeLibrary>', '<TreatWarningAsError>true</TreatWarningAsError>')) {
    if (-not $msixProject.Contains($required)) {
        throw "The MSIX Explorer command build is missing '$required'."
    }
}

if ($config.bundle.active -ne $true -or
    (@($config.bundle.targets) -join "`n") -cne 'nsis' -or
    $config.bundle.windows.nsis.installMode -cne 'currentUser' -or
    $config.bundle.windows.nsis.installerHooks -cne './windows/hooks.nsh') {
    throw 'Tauri bundling must produce only the current-user NSIS package with verified hooks.'
}
if (Test-Path -LiteralPath (Join-Path $projectRoot 'src-tauri\windows\shell-integration.wxs')) {
    throw 'The unsupported MSI Explorer integration definition must remain absent.'
}

Write-Output "MSIX and NSIS cover $($extensions.Count) video extensions and selected directories; MSI bundling is disabled."
