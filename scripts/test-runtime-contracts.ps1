$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot

& (Join-Path $PSScriptRoot 'setup-ffmpeg.ps1')
if ($LASTEXITCODE -ne 0) {
    throw "FFmpeg sidecar verification failed with exit code $LASTEXITCODE."
}

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
if ($package.engines.node -notlike "*$nodeVersion*") {
    throw "package.json does not constrain Node.js to the pinned .node-version value $nodeVersion."
}
$expectedNpmVersion = $package.packageManager -replace '^npm@', ''
$actualNpmVersion = (& npm --version).Trim()
if ($actualNpmVersion -ne $expectedNpmVersion) {
    throw "Active npm $actualNpmVersion does not match package.json packageManager $expectedNpmVersion."
}

$rustToolchain = Get-Content -LiteralPath (Join-Path $projectRoot 'rust-toolchain.toml') -Raw
$rustVersion = (& rustc --version).Split(' ')[1]
if ($rustToolchain -notmatch ('channel\s*=\s*"' + [regex]::Escape($rustVersion) + '"')) {
    throw "rust-toolchain.toml does not match the active Rust compiler $rustVersion."
}

Write-Output 'Runtime, sidecar, and workflow contracts are valid.'
