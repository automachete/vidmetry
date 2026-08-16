[CmdletBinding()]
param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $PSScriptRoot 'copyleft-sources.json'
$cargoManifestPath = Join-Path $projectRoot 'src-tauri\Cargo.toml'
$outputDirectory = Join-Path $projectRoot 'src-tauri\binaries\license-sources\MPL-2.0'

function Assert-Sha256([string]$Value, [string]$Description) {
    if ($Value -cnotmatch '^[0-9a-f]{64}$') {
        throw "$Description must be a lowercase SHA-256 value."
    }
}

function Get-ArchiveName([psobject]$Package) {
    return "$($Package.name)-$($Package.version).crate"
}

function Get-ArchiveUrl([psobject]$Package) {
    return "https://static.crates.io/crates/$($Package.name)/$(Get-ArchiveName $Package)"
}

function Test-ExpectedFile([string]$Path, [string]$ExpectedHash) {
    return (Test-Path -LiteralPath $Path -PathType Leaf) -and
        ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $ExpectedHash)
}

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Copyleft source manifest was not found: $manifestPath"
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.schemaVersion -ne 1 -or $manifest.license -cne 'MPL-2.0') {
    throw 'Copyleft source manifest schema or license is unsupported.'
}
Assert-Sha256 $manifest.licenseSource.sha256 'licenseSource.sha256'

$declared = @{}
foreach ($package in @($manifest.packages)) {
    if ($package.name -cnotmatch '^[a-z0-9][a-z0-9_-]*$' -or
        $package.version -cnotmatch '^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$') {
        throw "Invalid source-package identity: $($package.name) $($package.version)"
    }
    Assert-Sha256 $package.sha256 "$($package.name).sha256"
    $key = "$($package.name)@$($package.version)"
    if ($declared.ContainsKey($key)) { throw "Duplicate source-package identity: $key" }
    $declared[$key] = $package
}

$metadata = cargo metadata --format-version 1 --manifest-path $cargoManifestPath | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Unable to inspect Rust dependency licenses.' }
$actualMpl = @($metadata.packages | Where-Object {
    $_.license -and (($_.license -split '\s+(?:AND|OR|WITH)\s+|[()]') -contains 'MPL-2.0')
})
$actualKeys = @($actualMpl | ForEach-Object { "$($_.name)@$($_.version)" } | Sort-Object -Unique)
$declaredKeys = @($declared.Keys | Sort-Object)
if (($actualKeys -join "`n") -cne ($declaredKeys -join "`n")) {
    throw "MPL dependency/source manifest mismatch. Actual: $($actualKeys -join ', '); declared: $($declaredKeys -join ', ')"
}

New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
$licenseTarget = Join-Path $outputDirectory 'MPL-2.0.txt'
$indexTarget = Join-Path $outputDirectory 'SOURCE_INDEX.txt'
$expectedNames = @('MPL-2.0.txt', 'SOURCE_INDEX.txt') + @($manifest.packages | ForEach-Object { Get-ArchiveName $_ })
$unexpected = @(Get-ChildItem -LiteralPath $outputDirectory -Force | Where-Object {
    $_.PSIsContainer -or $_.Name -notin $expectedNames
})
if ($unexpected.Count -gt 0) {
    throw "Copyleft source directory contains unmanaged entries: $($unexpected.Name -join ', ')"
}

$indexLines = @(
    'Vidmetry bundled copyleft dependency sources',
    '',
    'License: MPL-2.0',
    'These unmodified crates.io source archives correspond to MPL-2.0 packages included in the Vidmetry executable:',
    ''
)
foreach ($package in @($manifest.packages)) {
    $indexLines += "- $(Get-ArchiveName $package)"
    $indexLines += "  Source: $(Get-ArchiveUrl $package)"
    $indexLines += "  SHA-256: $($package.sha256)"
}
$indexText = ($indexLines -join "`n") + "`n"

$sourcesValid = -not $Force -and (Test-ExpectedFile $licenseTarget $manifest.licenseSource.sha256)
foreach ($package in @($manifest.packages)) {
    $sourcesValid = $sourcesValid -and (Test-ExpectedFile (Join-Path $outputDirectory (Get-ArchiveName $package)) $package.sha256)
}
$sourcesValid = $sourcesValid -and (Test-Path -LiteralPath $indexTarget -PathType Leaf) -and
    ([IO.File]::ReadAllText($indexTarget) -ceq $indexText)
if ($sourcesValid) {
    Write-Output "Verified $($manifest.packages.Count) bundled MPL-2.0 source archives."
    exit 0
}

$operationRoot = Join-Path ([IO.Path]::GetTempPath()) "vidmetry-license-sources-$([Guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $operationRoot -Force | Out-Null
try {
    foreach ($package in @($manifest.packages)) {
        $archiveName = Get-ArchiveName $package
        $stagedArchive = Join-Path $operationRoot $archiveName
        & curl.exe --fail --location --retry 5 --retry-all-errors --connect-timeout 15 --max-time 300 --silent --show-error --output $stagedArchive (Get-ArchiveUrl $package)
        if ($LASTEXITCODE -ne 0) { throw "Unable to download source archive $archiveName." }
        if (-not (Test-ExpectedFile $stagedArchive $package.sha256)) {
            throw "Source archive checksum mismatch: $archiveName"
        }
    }

    $licensePackage = @($manifest.packages | Where-Object { $_.name -ceq $manifest.licenseSource.package })
    if ($licensePackage.Count -ne 1) { throw 'License source package is not uniquely declared.' }
    $licenseExtractRoot = Join-Path $operationRoot 'license'
    New-Item -ItemType Directory -Path $licenseExtractRoot -Force | Out-Null
    & tar.exe -xzf (Join-Path $operationRoot (Get-ArchiveName $licensePackage[0])) -C $licenseExtractRoot $manifest.licenseSource.archiveEntry
    if ($LASTEXITCODE -ne 0) { throw 'Unable to extract the MPL-2.0 license text.' }
    $stagedLicense = Join-Path $licenseExtractRoot ($manifest.licenseSource.archiveEntry -replace '/', [IO.Path]::DirectorySeparatorChar)
    if (-not (Test-ExpectedFile $stagedLicense $manifest.licenseSource.sha256)) {
        throw 'Extracted MPL-2.0 license checksum mismatch.'
    }

    foreach ($package in @($manifest.packages)) {
        Move-Item -LiteralPath (Join-Path $operationRoot (Get-ArchiveName $package)) -Destination (Join-Path $outputDirectory (Get-ArchiveName $package)) -Force
    }
    Copy-Item -LiteralPath $stagedLicense -Destination $licenseTarget -Force
    [IO.File]::WriteAllText($indexTarget, $indexText, [Text.UTF8Encoding]::new($false))
} finally {
    if (Test-Path -LiteralPath $operationRoot) {
        Remove-Item -LiteralPath $operationRoot -Recurse -Force
    }
}

Write-Output "Installed and verified $($manifest.packages.Count) bundled MPL-2.0 source archives."
