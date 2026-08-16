[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$toolsManifestPath = Join-Path $PSScriptRoot 'license-tools.json'
$aboutConfiguration = Join-Path $projectRoot 'about.toml'
$aboutTemplate = Join-Path $PSScriptRoot 'about.hbs'
$cargoManifest = Join-Path $projectRoot 'src-tauri\Cargo.toml'
$outputDirectory = Join-Path $projectRoot 'src-tauri\binaries\license-reports'
$rustReport = Join-Path $outputDirectory 'RUST_THIRD_PARTY_LICENSES.html'
$javascriptReport = Join-Path $outputDirectory 'JAVASCRIPT_THIRD_PARTY_LICENSES.txt'

function Assert-Sha256([string]$Value, [string]$Description) {
    if ($Value -cnotmatch '^[0-9a-f]{64}$') {
        throw "$Description must be a lowercase SHA-256 value."
    }
}

function Test-ExpectedFile([string]$Path, [string]$ExpectedHash) {
    return (Test-Path -LiteralPath $Path -PathType Leaf) -and
        ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $ExpectedHash)
}

$toolsManifest = Get-Content -LiteralPath $toolsManifestPath -Raw | ConvertFrom-Json
$cargoAbout = $toolsManifest.cargoAbout
if ($toolsManifest.schemaVersion -ne 1 -or $cargoAbout.version -cnotmatch '^\d+\.\d+\.\d+$') {
    throw 'License-tool manifest schema or version is invalid.'
}
Assert-Sha256 $cargoAbout.archiveSha256 'cargoAbout.archiveSha256'
Assert-Sha256 $cargoAbout.executableSha256 'cargoAbout.executableSha256'
$expectedUrl = "https://github.com/EmbarkStudios/cargo-about/releases/download/$($cargoAbout.version)/cargo-about-$($cargoAbout.version)-x86_64-pc-windows-msvc.tar.gz"
if ($cargoAbout.url -cne $expectedUrl) { throw 'cargo-about must use its immutable official Release URL.' }

$toolDirectory = Join-Path ([IO.Path]::GetTempPath()) "vidmetry-license-tools\cargo-about-$($cargoAbout.version)"
$cargoAboutExecutable = Join-Path $toolDirectory 'cargo-about.exe'
if (-not (Test-ExpectedFile $cargoAboutExecutable $cargoAbout.executableSha256)) {
    $operationRoot = Join-Path ([IO.Path]::GetTempPath()) "vidmetry-cargo-about-$([Guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $operationRoot -Force | Out-Null
    try {
        $archive = Join-Path $operationRoot 'cargo-about.tar.gz'
        & curl.exe --fail --location --retry 5 --retry-all-errors --connect-timeout 15 --max-time 300 --silent --show-error --output $archive $cargoAbout.url
        if ($LASTEXITCODE -ne 0 -or -not (Test-ExpectedFile $archive $cargoAbout.archiveSha256)) {
            throw 'Unable to download and authenticate cargo-about.'
        }
        & tar.exe -xzf $archive -C $operationRoot
        if ($LASTEXITCODE -ne 0) { throw 'Unable to extract cargo-about.' }
        $stagedExecutable = Get-ChildItem -LiteralPath $operationRoot -Filter cargo-about.exe -Recurse | Select-Object -First 1 -ExpandProperty FullName
        if (-not $stagedExecutable -or -not (Test-ExpectedFile $stagedExecutable $cargoAbout.executableSha256)) {
            throw 'Extracted cargo-about executable checksum mismatch.'
        }
        New-Item -ItemType Directory -Path $toolDirectory -Force | Out-Null
        Copy-Item -LiteralPath $stagedExecutable -Destination $cargoAboutExecutable -Force
    } finally {
        if (Test-Path -LiteralPath $operationRoot) {
            Remove-Item -LiteralPath $operationRoot -Recurse -Force
        }
    }
}

New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
& $cargoAboutExecutable generate --config $aboutConfiguration --manifest-path $cargoManifest --target x86_64-pc-windows-msvc --frozen --fail --output-file $rustReport $aboutTemplate
if ($LASTEXITCODE -ne 0) { throw 'Unable to generate the Rust third-party license report.' }

$licenseChecker = Join-Path $projectRoot 'node_modules\.bin\license-checker-rseidelsohn.cmd'
if (-not (Test-Path -LiteralPath $licenseChecker -PathType Leaf)) {
    throw 'JavaScript dependencies are not installed; run npm ci first.'
}
& $licenseChecker --production --excludePrivatePackages --plainVertical --out $javascriptReport
if ($LASTEXITCODE -ne 0) { throw 'Unable to generate the JavaScript third-party license report.' }

$rustText = Get-Content -LiteralPath $rustReport -Raw
$javascriptText = Get-Content -LiteralPath $javascriptReport -Raw
foreach ($required in @('cssparser', 'option-ext', 'tauri')) {
    if (-not $rustText.Contains($required, [StringComparison]::Ordinal)) {
        throw "Rust license report is missing required dependency $required."
    }
}
foreach ($required in @('@tauri-apps/api', 'i18next', 'svelte')) {
    if (-not $javascriptText.Contains($required, [StringComparison]::Ordinal)) {
        throw "JavaScript license report is missing required dependency $required."
    }
}

Write-Output 'Generated complete Rust and JavaScript third-party license reports.'
