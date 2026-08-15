param(
    [Parameter(Mandatory = $true)]
    [string]$Tag
)

$ErrorActionPreference = 'Stop'

if ($Tag -notmatch '^v(?<Version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$') {
    throw "Release tag must use the form v1.2.3 or v1.2.3-prerelease: $Tag"
}

$expectedVersion = $Matches.Version
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$packageVersion = (Get-Content -LiteralPath (Join-Path $repositoryRoot 'package.json') -Raw | ConvertFrom-Json).version
$tauriVersion = (Get-Content -LiteralPath (Join-Path $repositoryRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
$cargoContent = Get-Content -LiteralPath (Join-Path $repositoryRoot 'src-tauri/Cargo.toml') -Raw
$cargoMatch = [regex]::Match($cargoContent, '(?m)^version\s*=\s*"(?<Version>[^"]+)"')

if (-not $cargoMatch.Success) {
    throw 'Could not read the package version from src-tauri/Cargo.toml.'
}

$versions = [ordered]@{
    'package.json' = $packageVersion
    'src-tauri/Cargo.toml' = $cargoMatch.Groups['Version'].Value
    'src-tauri/tauri.conf.json' = $tauriVersion
}

foreach ($entry in $versions.GetEnumerator()) {
    if ($entry.Value -ne $expectedVersion) {
        throw "$($entry.Key) version '$($entry.Value)' does not match tag '$Tag'."
    }
}

Write-Host "Release tag $Tag matches all application version files."
