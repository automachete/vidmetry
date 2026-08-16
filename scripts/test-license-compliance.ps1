[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $PSScriptRoot 'ffmpeg-sidecars.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json

function Assert-FileContainsLiteral([string]$Path, [string]$Value, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Description file is missing: $Path"
    }
    $content = Get-Content -LiteralPath $Path -Raw
    if (-not $content.Contains($Value, [StringComparison]::Ordinal)) {
        throw "$Description does not contain required text: $Value"
    }
}

if ($manifest.engine.license -cne 'GPL-3.0-or-later' -or
    $manifest.engine.variant -cne 'win64-gpl') {
    throw 'FFmpeg must remain on the full GPL build so libx264 and libx265 are not removed.'
}
if ('--enable-nonfree' -cnotin @($manifest.engine.forbiddenConfigurationFlags)) {
    throw 'FFmpeg redistribution must reject nonfree configurations.'
}
foreach ($encoder in @('libx264', 'libx265', 'ffv1', 'aac')) {
    if ($encoder -cnotin @($manifest.engine.requiredEncoders)) {
        throw "Required application encoder is absent from the FFmpeg contract: $encoder"
    }
}
if ($manifest.archive.url -match '/latest/' -or
    $manifest.archive.url -notmatch '/releases/download/autobuild-\d{4}-\d{2}-\d{2}-\d{2}-\d{2}/') {
    throw 'FFmpeg must be downloaded from an immutable dated release asset.'
}
if ($manifest.correspondingSource.buildCommit -cnotmatch '^[0-9a-f]{40}$' -or
    $manifest.correspondingSource.ffmpegCommit -cnotmatch '^[0-9a-f]{40}$') {
    throw 'Complete corresponding source must pin full build and FFmpeg commits.'
}

$sourceScript = Join-Path $PSScriptRoot 'package-ffmpeg-corresponding-source.sh'
Assert-FileContainsLiteral $sourceScript 'env -u GITHUB_REPOSITORY ./download.sh' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript 'env -u GITHUB_REPOSITORY ./generate.sh win64 gpl' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript 'dependency_archives' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript 'git -C "$ffmpeg_checkout" archive' 'Corresponding-source packager'

$releaseWorkflow = Join-Path $projectRoot '.github\workflows\release.yml'
Assert-FileContainsLiteral $releaseWorkflow 'corresponding-source:' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'needs: corresponding-source' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'releaseDraft: true' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'gh release upload' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'gh release edit $tag --draft=false' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow $manifest.correspondingSource.archiveName 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'setup-copyleft-sources.ps1' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'generate-third-party-licenses.ps1' 'Release workflow'

$sourceAuditWorkflow = Join-Path $projectRoot '.github\workflows\ffmpeg-source-audit.yml'
Assert-FileContainsLiteral $sourceAuditWorkflow 'package-ffmpeg-corresponding-source.sh' 'Corresponding-source audit workflow'
Assert-FileContainsLiteral $sourceAuditWorkflow 'tar -tf' 'Corresponding-source audit workflow'

$ciWorkflow = Join-Path $projectRoot '.github\workflows\ci.yml'
Assert-FileContainsLiteral $ciWorkflow 'EmbarkStudios/cargo-deny-action@' 'CI workflow'
Assert-FileContainsLiteral $ciWorkflow 'npm run check:licenses' 'CI workflow'
Assert-FileContainsLiteral $ciWorkflow 'npm run test:licenses' 'CI workflow'
Assert-FileContainsLiteral $ciWorkflow 'setup-copyleft-sources.ps1' 'CI workflow'
Assert-FileContainsLiteral $ciWorkflow 'generate-third-party-licenses.ps1' 'CI workflow'

$tauriConfiguration = Join-Path $projectRoot 'src-tauri\tauri.conf.json'
Assert-FileContainsLiteral $tauriConfiguration '"binaries/ffmpeg-notices": "FFmpeg"' 'Tauri bundle configuration'
Assert-FileContainsLiteral $tauriConfiguration '"binaries/license-reports": "ThirdPartyLicenses"' 'Tauri bundle configuration'
Assert-FileContainsLiteral $tauriConfiguration '"binaries/license-sources": "LicenseSources"' 'Tauri bundle configuration'

$copyleftManifestPath = Join-Path $PSScriptRoot 'copyleft-sources.json'
$copyleftManifest = Get-Content -LiteralPath $copyleftManifestPath -Raw | ConvertFrom-Json
if ($copyleftManifest.license -cne 'MPL-2.0') {
    throw 'Copyleft dependency source manifest must cover MPL-2.0.'
}
$metadata = cargo metadata --format-version 1 --manifest-path (Join-Path $projectRoot 'src-tauri\Cargo.toml') | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Unable to inspect Rust dependency licenses.' }
$actualMplPackages = @($metadata.packages | Where-Object {
    $_.license -and (($_.license -split '\s+(?:AND|OR|WITH)\s+|[()]') -contains 'MPL-2.0')
} | ForEach-Object { "$($_.name)@$($_.version)" } | Sort-Object -Unique)
$declaredMplPackages = @($copyleftManifest.packages | ForEach-Object { "$($_.name)@$($_.version)" } | Sort-Object -Unique)
if (($actualMplPackages -join "`n") -cne ($declaredMplPackages -join "`n")) {
    throw 'Every MPL-2.0 Rust dependency must have an exact bundled Source Form archive.'
}
$copyleftSourceDirectory = Join-Path $projectRoot 'src-tauri\binaries\license-sources\MPL-2.0'
foreach ($copyleftPackage in @($copyleftManifest.packages)) {
    $sourceArchive = Join-Path $copyleftSourceDirectory "$($copyleftPackage.name)-$($copyleftPackage.version).crate"
    if (-not (Test-Path -LiteralPath $sourceArchive -PathType Leaf) -or
        (Get-FileHash -LiteralPath $sourceArchive -Algorithm SHA256).Hash.ToLowerInvariant() -cne $copyleftPackage.sha256) {
        throw "Bundled MPL-2.0 source archive is missing or invalid: $($copyleftPackage.name)"
    }
}
Assert-FileContainsLiteral (Join-Path $copyleftSourceDirectory 'MPL-2.0.txt') 'Mozilla Public License Version 2.0' 'Bundled MPL license'

$licenseToolsManifest = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'license-tools.json') -Raw | ConvertFrom-Json
if ($licenseToolsManifest.cargoAbout.url -match '/latest/' -or
    $licenseToolsManifest.cargoAbout.url -notmatch '/releases/download/\d+\.\d+\.\d+/') {
    throw 'Third-party notice generation must use an immutable cargo-about Release.'
}
$licenseReportsDirectory = Join-Path $projectRoot 'src-tauri\binaries\license-reports'
Assert-FileContainsLiteral (Join-Path $licenseReportsDirectory 'RUST_THIRD_PARTY_LICENSES.html') 'cssparser' 'Rust third-party license report'
Assert-FileContainsLiteral (Join-Path $licenseReportsDirectory 'RUST_THIRD_PARTY_LICENSES.html') 'Mozilla Public License 2.0' 'Rust third-party license report'
Assert-FileContainsLiteral (Join-Path $licenseReportsDirectory 'JAVASCRIPT_THIRD_PARTY_LICENSES.txt') '@tauri-apps/api' 'JavaScript third-party license report'

$notices = Join-Path $projectRoot 'THIRD_PARTY_NOTICES.md'
Assert-FileContainsLiteral $notices $manifest.engine.id 'Third-party notices'
Assert-FileContainsLiteral $notices $manifest.correspondingSource.archiveName 'Third-party notices'
Assert-FileContainsLiteral $notices $manifest.correspondingSource.buildCommit 'Third-party notices'
Assert-FileContainsLiteral $notices $manifest.correspondingSource.ffmpegCommit 'Third-party notices'
Assert-FileContainsLiteral $notices 'Rust packages under MPL-2.0' 'Third-party notices'
Assert-FileContainsLiteral $notices 'ThirdPartyLicenses' 'Third-party notices'

$license = Get-Content -LiteralPath (Join-Path $projectRoot 'LICENSE') -Raw
if (-not $license.StartsWith('MIT License', [StringComparison]::Ordinal)) {
    throw 'Vidmetry application license must remain MIT and distinct from the GPL sidecars.'
}
$cargoManifest = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri\Cargo.toml') -Raw
if ($cargoManifest -cnotmatch '(?m)^license = "MIT"$') {
    throw 'The Rust package must declare the Vidmetry MIT license.'
}
$package = Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json
if ($package.license -cne 'MIT') {
    throw 'The JavaScript package must declare the Vidmetry MIT license.'
}

$sourceNotice = Join-Path $projectRoot "src-tauri\binaries\ffmpeg-notices\$($manifest.notices.correspondingSourceFileName)"
$expectedSourceUrl = "$($manifest.correspondingSource.officialReleaseBaseUrl)/v$($package.version)/$($manifest.correspondingSource.archiveName)"
Assert-FileContainsLiteral $sourceNotice $expectedSourceUrl 'Installed FFmpeg source notice'

Write-Output 'Application, dependency-license, FFmpeg source, installer notice, and fail-closed release contracts are valid.'
