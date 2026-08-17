$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$selectionPath = Join-Path $projectRoot 'src-tauri\src\selection.rs'
$hookPath = Join-Path $projectRoot 'src-tauri\windows\hooks.nsh'
$wixPath = Join-Path $projectRoot 'src-tauri\windows\shell-integration.wxs'
$configPath = Join-Path $projectRoot 'src-tauri\tauri.conf.json'
$msixManifestPath = Join-Path $projectRoot 'scripts\msix\AppxManifest.xml.template'
$msixBuildPath = Join-Path $projectRoot 'scripts\build-msix.ps1'
$msixCommandPath = Join-Path $projectRoot 'src-tauri\windows\msix-explorer-command\ExplorerCommand.cpp'
$msixProjectPath = Join-Path $projectRoot 'src-tauri\windows\msix-explorer-command\ExplorerCommand.vcxproj'

$selection = Get-Content -LiteralPath $selectionPath -Raw
$hook = Get-Content -LiteralPath $hookPath -Raw
$wix = Get-Content -LiteralPath $wixPath -Raw
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
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
    if (-not $hook.Contains($register) -or -not $hook.Contains($unregister)) {
        throw "The NSIS Explorer integration does not cover .$extension."
    }
    if (-not $wix.Contains("Software\Classes\.$extension\OpenWithProgids") -or
        -not $wix.Contains("Name=`".$extension`"")) {
        throw "The MSI Explorer integration does not cover .$extension."
    }
}

$requiredHookContent = @(
    'Software\Classes\Vidmetry.Video',
    'Software\Classes\Directory\shell\Vidmetry',
    'OpenWithProgids',
    'ExplorerIntegrationEnabled',
    'NSIS_HOOK_POSTINSTALL',
    'NSIS_HOOK_POSTUNINSTALL',
    'SHChangeNotify'
)
foreach ($required in $requiredHookContent) {
    if (-not $hook.Contains($required)) {
        throw "The NSIS Explorer integration is missing '$required'."
    }
}

$requiredWixContent = @(
    'Software\Classes\Vidmetry.Video',
    'Software\Classes\Directory\shell\Vidmetry',
    'VIDMETRY_EXPLORER_INTEGRATION_ENABLED',
    'Transitive="yes"'
)
foreach ($required in $requiredWixContent) {
    if (-not $wix.Contains($required)) {
        throw "The MSI Explorer integration is missing '$required'."
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

if ($config.bundle.windows.nsis.installMode -ne 'currentUser') {
    throw 'Explorer integration requires the NSIS current-user install mode.'
}
if ($config.bundle.windows.nsis.installerHooks -ne './windows/hooks.nsh') {
    throw 'The NSIS Explorer integration hook is not configured.'
}
if ($config.bundle.windows.wix.fragmentPaths -notcontains './windows/shell-integration.wxs' -or
    $config.bundle.windows.wix.componentGroupRefs -notcontains 'VidmetryExplorerIntegration') {
    throw 'The MSI Explorer integration fragment is not configured.'
}

Write-Output "MSI, NSIS, and MSIX cover $($extensions.Count) video extensions and selected directories."
