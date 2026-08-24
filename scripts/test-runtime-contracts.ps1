$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot

& (Join-Path $PSScriptRoot 'setup-ffmpeg.ps1')

$workflowDirectory = Join-Path $projectRoot '.github\workflows'
foreach ($workflow in Get-ChildItem -LiteralPath $workflowDirectory -Filter '*.yml') {
    $workflowText = Get-Content -LiteralPath $workflow.FullName -Raw
    if ($workflowText -match '(?m)^\s*pull_request_target\s*:') {
        throw "pull_request_target is forbidden because it can expose privileged workflow context to untrusted pull request code: $($workflow.Name)"
    }

    foreach ($line in Get-Content -LiteralPath $workflow.FullName) {
        if ($line -match '^\s*uses:\s+([^\s]+)') {
            $reference = $Matches[1]
            if ($reference -notmatch '@[0-9a-f]{40}$') {
                throw "GitHub Action references must be pinned to a full commit SHA: $($workflow.Name): $reference"
            }
        }
    }
}

$codeOwnersPath = Join-Path $projectRoot '.github\CODEOWNERS'
$codeOwners = @(Get-Content -LiteralPath $codeOwnersPath)
foreach ($requiredEntry in @('/.github/workflows/** @automachete', '/.github/CODEOWNERS @automachete')) {
    if ($requiredEntry -cnotin $codeOwners) {
        throw "CODEOWNERS must contain: $requiredEntry"
    }
}

$releaseWorkflow = Get-Content -LiteralPath (Join-Path $workflowDirectory 'release.yml') -Raw
if ($releaseWorkflow -notmatch '(?m)^env:\r?\n  GH_REPO: \$\{\{ github\.repository \}\}\s*$') {
    throw 'The Release workflow must set GH_REPO for checkout-free gh commands.'
}
if ($releaseWorkflow -notmatch '(?m)^\s{4}environment:\s+microsoft-store-release\s*$') {
    throw 'The Microsoft Store MSIX job must use the microsoft-store-release environment.'
}

$tauriConfiguration = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$mainWindows = @($tauriConfiguration.app.windows | Where-Object label -ceq 'main')
if ($mainWindows.Count -ne 1) {
    throw 'Tauri must define exactly one main application window.'
}
$mainWindow = $mainWindows[0]
if ($mainWindow.width -ne 1280 -or $mainWindow.height -ne 900 -or
    $mainWindow.minWidth -ne 960 -or $mainWindow.minHeight -ne 720) {
    throw 'The main window must use the verified 1280x900 default and 960x720 minimum layout.'
}

$package = Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json
$nodeVersion = (Get-Content -LiteralPath (Join-Path $projectRoot '.node-version') -Raw).Trim()
$parsedNodeVersion = [Version]$nodeVersion
$expectedNodeRange = ">=$nodeVersion <$($parsedNodeVersion.Major + 1)"
if ($package.engines.node -cne $expectedNodeRange) {
    throw "package.json Node.js engine must be exactly '$expectedNodeRange'."
}
$actualNodeVersion = (& node --version).Trim().TrimStart('v')
if ($actualNodeVersion -cne $nodeVersion) {
    throw "Active Node.js $actualNodeVersion does not match .node-version $nodeVersion."
}
if ($package.packageManager -notmatch '^npm@(?<version>\d+\.\d+\.\d+)$') {
    throw 'package.json packageManager must pin an exact npm semantic version.'
}
$expectedNpmVersion = $Matches.version
$parsedNpmVersion = [Version]$expectedNpmVersion
$expectedNpmRange = ">=$expectedNpmVersion <$($parsedNpmVersion.Major + 1)"
if ($package.engines.npm -cne $expectedNpmRange) {
    throw "package.json npm engine must be exactly '$expectedNpmRange'."
}
$actualNpmVersion = (& npm --version).Trim()
if ($actualNpmVersion -ne $expectedNpmVersion) {
    throw "Active npm $actualNpmVersion does not match package.json packageManager $expectedNpmVersion."
}

$rustToolchain = Get-Content -LiteralPath (Join-Path $projectRoot 'rust-toolchain.toml') -Raw
$channelMatches = [regex]::Matches($rustToolchain, '(?m)^\s*channel\s*=\s*"(?<version>[^"]+)"\s*$')
if ($channelMatches.Count -ne 1) {
    throw 'rust-toolchain.toml must define exactly one channel.'
}
$expectedRustVersion = $channelMatches[0].Groups['version'].Value
$rustVersion = (& rustc --version).Split(' ')[1]
if ($rustVersion -cne $expectedRustVersion) {
    throw "Active Rust $rustVersion does not match rust-toolchain.toml $expectedRustVersion."
}

Write-Output 'Runtime, sidecar, and workflow contracts are valid.'
