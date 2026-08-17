$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$selectionPath = Join-Path $projectRoot 'src-tauri\src\selection.rs'
$hookPath = Join-Path $projectRoot 'src-tauri\windows\hooks.nsh'
$wixPath = Join-Path $projectRoot 'src-tauri\windows\shell-integration.wxs'
$configPath = Join-Path $projectRoot 'src-tauri\tauri.conf.json'

$selection = Get-Content -LiteralPath $selectionPath -Raw
$hook = Get-Content -LiteralPath $hookPath -Raw
$wix = Get-Content -LiteralPath $wixPath -Raw
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json

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

Write-Output "Both installers cover $($extensions.Count) video extensions and selected directories."
