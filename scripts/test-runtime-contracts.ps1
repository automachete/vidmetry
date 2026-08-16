$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot

& (Join-Path $PSScriptRoot 'setup-ffmpeg.ps1')

$workflowDirectory = Join-Path $projectRoot '.github\workflows'
foreach ($workflow in Get-ChildItem -LiteralPath $workflowDirectory -Filter '*.yml') {
    foreach ($line in Get-Content -LiteralPath $workflow.FullName) {
        if ($line -match '^\s*uses:\s+([^\s]+)') {
            $reference = $Matches[1]
            if ($reference -notmatch '@[0-9a-f]{40}$') {
                throw "GitHub Action references must be pinned to a full commit SHA: $($workflow.Name): $reference"
            }
        }
    }
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
