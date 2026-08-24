param(
    [Parameter(Mandatory = $true)]
    [string]$Tag,

    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ($Tag -notmatch '^v(?<Version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$') {
    throw "Release tag must use the form v1.2.3 or v1.2.3-prerelease: $Tag"
}

$expectedVersion = $Matches.Version
$repositoryRoot = [System.IO.Path]::GetFullPath($RepositoryRoot)
$packageVersion = (Get-Content -LiteralPath (Join-Path $repositoryRoot 'package.json') -Raw | ConvertFrom-Json).version
$packageLock = Get-Content -LiteralPath (Join-Path $repositoryRoot 'package-lock.json') -Raw | ConvertFrom-Json -AsHashtable
$tauriVersion = (Get-Content -LiteralPath (Join-Path $repositoryRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
$cargoContent = Get-Content -LiteralPath (Join-Path $repositoryRoot 'src-tauri/Cargo.toml') -Raw
$cargoMatch = [regex]::Match($cargoContent, '(?m)^version\s*=\s*"(?<Version>[^"]+)"')
$cargoLockContent = Get-Content -LiteralPath (Join-Path $repositoryRoot 'src-tauri/Cargo.lock') -Raw
$cargoLockMatch = [regex]::Match(
    $cargoLockContent,
    '(?ms)^\[\[package\]\]\r?\nname\s*=\s*"vidmetry"\r?\nversion\s*=\s*"(?<Version>[^"]+)"'
)

if (-not $cargoMatch.Success) {
    throw 'Could not read the package version from src-tauri/Cargo.toml.'
}
if (-not $cargoLockMatch.Success) {
    throw 'Could not read the package version from src-tauri/Cargo.lock.'
}

$versions = [ordered]@{
    'package.json' = $packageVersion
    'package-lock.json root' = $packageLock['version']
    'package-lock.json workspace' = $packageLock['packages']['']['version']
    'src-tauri/Cargo.toml' = $cargoMatch.Groups['Version'].Value
    'src-tauri/Cargo.lock' = $cargoLockMatch.Groups['Version'].Value
    'src-tauri/tauri.conf.json' = $tauriVersion
}

foreach ($entry in $versions.GetEnumerator()) {
    if ($entry.Value -ne $expectedVersion) {
        throw "$($entry.Key) version '$($entry.Value)' does not match tag '$Tag'."
    }
}

$documentContracts = [ordered]@{
    'docs/SDD.md product version' = '(?m)^\| Product version \| (?<Version>[0-9A-Za-z.-]+) \|\r?$'
    'docs/SDD.md non-goals version' = '(?m)^### 2\.2 Non-goals for (?<Version>[0-9A-Za-z.-]+)\r?$'
    'docs/SDD.md acceptance heading version' = '(?m)^## 14\. Acceptance criteria for (?<Version>[0-9A-Za-z.-]+)\r?$'
    'docs/SDD.md acceptance summary version' = '(?m)^The (?<Version>[0-9A-Za-z.-]+) implementation satisfies AC-001'
    'docs/VERIFICATION.md title version' = '(?m)^# Vidmetry (?<Version>[0-9A-Za-z.-]+) Verification Record\r?$'
    'docs/VERIFICATION.md source-tree version' = 'generated from the verified (?<Version>[0-9A-Za-z.-]+) source tree'
    'docs/VERIFICATION.md MSIX artifact version' = '`Vidmetry_(?<Version>[0-9A-Za-z.-]+)\.0_x64\.msix`'
}

foreach ($contract in $documentContracts.GetEnumerator()) {
    $relativePath = $contract.Key.Split(' ')[0]
    $content = Get-Content -LiteralPath (Join-Path $repositoryRoot $relativePath) -Raw
    $matches = [regex]::Matches($content, $contract.Value)
    if ($matches.Count -ne 1) {
        throw "$($contract.Key) must occur exactly once; found $($matches.Count)."
    }
    if ($matches[0].Groups['Version'].Value -cne $expectedVersion) {
        throw "$($contract.Key) '$($matches[0].Groups['Version'].Value)' does not match tag '$Tag'."
    }
}

Write-Host "Release tag $Tag matches all application and documentation version fields."
