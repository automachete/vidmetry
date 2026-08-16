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

function Assert-FileDoesNotContainLiteral([string]$Path, [string]$Value, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Description file is missing: $Path"
    }
    $content = Get-Content -LiteralPath $Path -Raw
    if ($content.Contains($Value, [StringComparison]::Ordinal)) {
        throw "$Description contains forbidden text: $Value"
    }
}
if ($manifest.correspondingSource.archiveSha256 -cnotmatch '^[0-9a-f]{64}$' -or
    $manifest.correspondingSource.assetTag -cnotmatch '^ffmpeg-source-[A-Za-z0-9.-]+$') {
    throw 'Complete corresponding source must pin an immutable archive checksum and engine asset tag.'
}

$sourceScript = Join-Path $PSScriptRoot 'package-ffmpeg-corresponding-source.sh'
Assert-FileContainsLiteral $sourceScript 'env -u GITHUB_REPOSITORY ./download.sh' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript 'env -u GITHUB_REPOSITORY ./generate.sh win64 gpl' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript 'dependency_archives' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript 'git -C "$ffmpeg_checkout" archive' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript 'actual_archive_sha256' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript 'repack-source-directory.sh' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript 'archive-source-tree.sh' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript '--network none --read-only --cap-drop ALL' 'Corresponding-source packager'
Assert-FileContainsLiteral $sourceScript '--security-opt no-new-privileges' 'Corresponding-source packager'
$sourceArchiver = Join-Path $PSScriptRoot 'archive-source-tree.sh'
Assert-FileContainsLiteral $sourceArchiver 'SOURCE_SHA256SUMS' 'Corresponding-source archiver'
Assert-FileContainsLiteral $sourceArchiver '--format=gnu --sort=name' 'Corresponding-source archiver'
Assert-FileContainsLiteral $sourceArchiver 'sanitize-source-tree.sh' 'Corresponding-source archiver'

$sourceTreeSanitizer = Join-Path $PSScriptRoot 'sanitize-source-tree.sh'
Assert-FileContainsLiteral $sourceTreeSanitizer '! -type d ! -type f ! -type l' 'Corresponding-source tree sanitizer'
Assert-FileContainsLiteral $sourceTreeSanitizer '-type f -links +1' 'Corresponding-source tree sanitizer'
Assert-FileContainsLiteral $sourceTreeSanitizer 'removed-external-symlinks.tsv' 'Corresponding-source tree sanitizer'

$sidecarManifest = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'ffmpeg-sidecars.json') -Raw | ConvertFrom-Json
if ($sidecarManifest.correspondingSource.packagingImage -notmatch '^ghcr\.io/[a-z0-9._/-]+@sha256:[0-9a-f]{64}$') {
    throw 'Corresponding-source packaging image must be pinned by OCI digest.'
}
Assert-FileContainsLiteral (Join-Path $PSScriptRoot 'setup-ffmpeg.ps1') 'Source packaging image: $($Manifest.correspondingSource.packagingImage)' 'FFmpeg build notice generator'

$sourceRepacker = Join-Path $PSScriptRoot 'repack-source-archive.sh'
Assert-FileContainsLiteral $sourceRepacker '--format=gnu --sort=name' 'Dependency-source normalizer'
Assert-FileContainsLiteral $sourceRepacker "-name .git" 'Dependency-source normalizer'
Assert-FileContainsLiteral $sourceRepacker "--mtime='UTC 1970-01-01'" 'Dependency-source normalizer'
Assert-FileContainsLiteral $sourceRepacker 'sanitize-source-tree.sh' 'Dependency-source normalizer'
$sourceDirectoryRepacker = Join-Path $PSScriptRoot 'repack-source-directory.sh'
Assert-FileContainsLiteral $sourceDirectoryRepacker 'repack-source-archive.sh' 'Dependency-source directory normalizer'
Assert-FileContainsLiteral $sourceDirectoryRepacker 'wait -n' 'Dependency-source directory normalizer'
$sourceRepackerTest = Join-Path $PSScriptRoot 'test-source-repacker.sh'
Assert-FileContainsLiteral $sourceRepackerTest 'cmp "$work_root/one-normalized.tar.xz" "$work_root/two-normalized.tar.xz"' 'Dependency-source normalizer test'
Assert-FileContainsLiteral $sourceRepackerTest 'repack-source-directory.sh' 'Dependency-source normalizer test'
Assert-FileContainsLiteral $sourceRepackerTest 'archive-source-tree.sh' 'Corresponding-source archiver test'
Assert-FileContainsLiteral $sourceRepackerTest 'external-normalized.tar.xz' 'Corresponding-source boundary test'
Assert-FileContainsLiteral $sourceRepackerTest 'drive-relative-escape' 'Corresponding-source boundary test'
Assert-FileContainsLiteral $sourceRepackerTest 'special-normalized.tar.xz' 'Corresponding-source boundary test'
Assert-FileContainsLiteral $sourceRepackerTest 'hard-link-normalized.tar.xz' 'Corresponding-source boundary test'
Assert-FileContainsLiteral $sourceRepackerTest 'outer-absolute.tar.xz' 'Corresponding-source boundary test'

$releaseWorkflow = Join-Path $projectRoot '.github\workflows\release.yml'
Assert-FileContainsLiteral $releaseWorkflow 'preflight:' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'source-assets:' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'build-windows:' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'needs: [preflight, source-assets, build-windows]' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'gh release download "$ASSET_TAG"' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'gh release upload' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'gh release edit $tag --draft=false' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'RELEASE_TAG: ${{ github.ref_name }}' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'RELEASE_REPOSITORY: ${{ github.repository }}' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'visibility="$(gh api "repos/$RELEASE_REPOSITORY"' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'Public repository access could not be confirmed immediately before publication.' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow '.correspondingSource.archiveSha256' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow '.correspondingSource.assetTag' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'EmbarkStudios/cargo-deny-action@' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'persist-credentials: false' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'setup-copyleft-sources.ps1' 'Release workflow'
Assert-FileContainsLiteral $releaseWorkflow 'generate-third-party-licenses.ps1' 'Release workflow'
Assert-FileDoesNotContainLiteral $releaseWorkflow 'package-ffmpeg-corresponding-source.sh' 'Release workflow'
$releaseWorkflowText = Get-Content -LiteralPath $releaseWorkflow -Raw
foreach ($unsafeExpansion in @('-Tag "${{ github.ref_name }}"', '$tag = "${{ github.ref_name }}"')) {
    if ($releaseWorkflowText.Contains($unsafeExpansion, [StringComparison]::Ordinal)) {
        throw 'Release event values must enter PowerShell through environment variables, not expression interpolation.'
    }
}

$sourceAuditWorkflow = Join-Path $projectRoot '.github\workflows\ffmpeg-source-audit.yml'
Assert-FileContainsLiteral $sourceAuditWorkflow 'package-ffmpeg-corresponding-source.sh' 'Corresponding-source audit workflow'
Assert-FileContainsLiteral $sourceAuditWorkflow 'scripts/test-source-repacker.sh' 'Corresponding-source audit workflow'
Assert-FileContainsLiteral $sourceAuditWorkflow 'tar -tf' 'Corresponding-source audit workflow'
Assert-FileContainsLiteral $sourceAuditWorkflow '.correspondingSource.archiveSha256' 'Corresponding-source audit workflow'
Assert-FileDoesNotContainLiteral $sourceAuditWorkflow 'actions/upload-artifact@' 'Corresponding-source audit workflow'

$sourceReleaseWorkflow = Join-Path $projectRoot '.github\workflows\ffmpeg-source-release.yml'
Assert-FileContainsLiteral $sourceReleaseWorkflow 'branches: [main]' 'Immutable source Release workflow'
Assert-FileContainsLiteral $sourceReleaseWorkflow 'Reuse existing immutable source release' 'Immutable source Release workflow'
Assert-FileContainsLiteral $sourceReleaseWorkflow 'build-source:' 'Immutable source Release workflow'
Assert-FileContainsLiteral $sourceReleaseWorkflow 'publish-source:' 'Immutable source Release workflow'
Assert-FileContainsLiteral $sourceReleaseWorkflow 'needs: [inspect-source, build-source]' 'Immutable source Release workflow'
Assert-FileContainsLiteral $sourceReleaseWorkflow 'persist-credentials: false' 'Immutable source Release workflow'
Assert-FileContainsLiteral $sourceReleaseWorkflow 'Transfer verified source to isolated publisher' 'Immutable source Release workflow'
Assert-FileContainsLiteral $sourceReleaseWorkflow 'gh release create $env:ASSET_TAG' 'Immutable source Release workflow'
Assert-FileContainsLiteral $sourceReleaseWorkflow '--latest=false' 'Immutable source Release workflow'
Assert-FileContainsLiteral $sourceReleaseWorkflow 'gh release edit $env:ASSET_TAG --draft=false' 'Immutable source Release workflow'
Assert-FileDoesNotContainLiteral $sourceReleaseWorkflow '--clobber' 'Immutable source Release workflow'
$sourceReleaseWorkflowText = Get-Content -LiteralPath $sourceReleaseWorkflow -Raw
$buildSourceJob = [regex]::Match(
    $sourceReleaseWorkflowText,
    '(?ms)^  build-source:\r?\n(?<Body>.*?)(?=^  publish-source:)'
)
$publishSourceJob = [regex]::Match(
    $sourceReleaseWorkflowText,
    '(?ms)^  publish-source:\r?\n(?<Body>.*)\z'
)
if (-not $buildSourceJob.Success -or -not $publishSourceJob.Success) {
    throw 'Immutable source Release workflow jobs could not be parsed for privilege isolation.'
}
$buildSourceBody = $buildSourceJob.Groups['Body'].Value
$publishSourceBody = $publishSourceJob.Groups['Body'].Value
if (-not $buildSourceBody.Contains('contents: read', [StringComparison]::Ordinal) -or
    $buildSourceBody.Contains('contents: write', [StringComparison]::Ordinal) -or
    $buildSourceBody.Contains('secrets.GITHUB_TOKEN', [StringComparison]::Ordinal)) {
    throw 'Source assembly must run with read-only contents permission and without a publication token.'
}
if (-not $publishSourceBody.Contains('contents: write', [StringComparison]::Ordinal) -or
    $publishSourceBody.Contains('package-ffmpeg-corresponding-source.sh', [StringComparison]::Ordinal) -or
    $publishSourceBody.Contains('actions/checkout@', [StringComparison]::Ordinal)) {
    throw 'Source publication must receive verified artifacts without checking out or rebuilding source.'
}

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
