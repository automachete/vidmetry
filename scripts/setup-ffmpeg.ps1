[CmdletBinding()]
param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$binariesDirectory = Join-Path $projectRoot 'src-tauri\binaries'
$noticesDirectory = Join-Path $binariesDirectory 'ffmpeg-notices'
$manifestPath = Join-Path $PSScriptRoot 'ffmpeg-sidecars.json'

function Assert-Sha256([string]$Value, [string]$Name) {
    if ($Value -notmatch '^[0-9a-f]{64}$') {
        throw "$Name must be a lowercase SHA-256 value."
    }
}

function Assert-ExactProperties([psobject]$Value, [string[]]$Expected, [string]$Name) {
    $actual = @($Value.PSObject.Properties.Name)
    $missing = @($Expected | Where-Object { $_ -notin $actual })
    $extra = @($actual | Where-Object { $_ -notin $Expected })
    if ($missing.Count -gt 0 -or $extra.Count -gt 0) {
        throw "$Name has an invalid shape. Missing: $($missing -join ', '); extra: $($extra -join ', ')."
    }
}

function Test-ExpectedFile([string]$Path, [string]$ExpectedHash) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $false
    }
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() -eq $ExpectedHash
}

function Resolve-ArchiveFile([string]$ExtractRoot, [string]$RelativePath) {
    $candidate = [IO.Path]::GetFullPath((Join-Path $ExtractRoot ($RelativePath -replace '/', [IO.Path]::DirectorySeparatorChar)))
    $rootPrefix = [IO.Path]::GetFullPath($ExtractRoot).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $candidate.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Archive entry escapes the extraction directory: $RelativePath"
    }
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        throw "The FFmpeg archive is missing the required file: $RelativePath"
    }
    return $candidate
}

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "FFmpeg sidecar manifest was not found: $manifestPath"
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
Assert-ExactProperties $manifest @('version', 'archive', 'targets', 'notices') 'manifest'
Assert-ExactProperties $manifest.archive @('url', 'sha256') 'archive'
Assert-ExactProperties $manifest.notices @('license', 'buildInfo') 'notices'
Assert-ExactProperties $manifest.notices.license @('archivePath', 'sha256', 'fileName') 'notices.license'
Assert-ExactProperties $manifest.notices.buildInfo @('archivePath', 'sha256', 'fileName') 'notices.buildInfo'
if ($manifest.notices.license.fileName -cne 'FFMPEG_LICENSE.txt' -or
    $manifest.notices.buildInfo.fileName -cne 'FFMPEG_BUILD_INFO.txt') {
    throw 'FFmpeg notice output names must match the bundled notice contract.'
}
if ($manifest.version -notmatch '^\d+\.\d+\.\d+$') {
    throw 'FFmpeg manifest version must be an exact semantic version.'
}
$expectedArchiveUrl = "https://github.com/GyanD/codexffmpeg/releases/download/$($manifest.version)/ffmpeg-$($manifest.version)-essentials_build.zip"
if ($manifest.archive.url -cne $expectedArchiveUrl) {
    throw 'FFmpeg archive URL must identify the immutable versioned GitHub release asset.'
}
Assert-Sha256 $manifest.archive.sha256 'archive.sha256'

$rustc = Get-Command rustc -CommandType Application -ErrorAction Stop
$targetTriple = (& $rustc.Source --print host-tuple).Trim()
if ([string]::IsNullOrWhiteSpace($targetTriple)) {
    throw 'Unable to determine the Rust host target triple.'
}
$targetProperty = $manifest.targets.PSObject.Properties[$targetTriple]
if ($null -eq $targetProperty) {
    throw "FFmpeg sidecars are not defined for Rust target $targetTriple."
}
$target = $targetProperty.Value
Assert-ExactProperties $target @('ffmpeg', 'ffprobe') $targetTriple
Assert-ExactProperties $target.ffmpeg @('archivePath', 'sha256') "$targetTriple.ffmpeg"
Assert-ExactProperties $target.ffprobe @('archivePath', 'sha256') "$targetTriple.ffprobe"
Assert-Sha256 $target.ffmpeg.sha256 "$targetTriple.ffmpeg.sha256"
Assert-Sha256 $target.ffprobe.sha256 "$targetTriple.ffprobe.sha256"
Assert-Sha256 $manifest.notices.license.sha256 'notices.license.sha256'
Assert-Sha256 $manifest.notices.buildInfo.sha256 'notices.buildInfo.sha256'

New-Item -ItemType Directory -Path $binariesDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $noticesDirectory -Force | Out-Null
$ffmpegTarget = Join-Path $binariesDirectory "ffmpeg-$targetTriple.exe"
$ffprobeTarget = Join-Path $binariesDirectory "ffprobe-$targetTriple.exe"
$licenseTarget = Join-Path $noticesDirectory $manifest.notices.license.fileName
$buildInfoTarget = Join-Path $noticesDirectory $manifest.notices.buildInfo.fileName
$installedFiles = @(
    @{ Path = $ffmpegTarget; Hash = $target.ffmpeg.sha256 },
    @{ Path = $ffprobeTarget; Hash = $target.ffprobe.sha256 },
    @{ Path = $licenseTarget; Hash = $manifest.notices.license.sha256 },
    @{ Path = $buildInfoTarget; Hash = $manifest.notices.buildInfo.sha256 }
)
$expectedNoticeNames = @($manifest.notices.license.fileName, $manifest.notices.buildInfo.fileName)
$unexpectedNotices = @(Get-ChildItem -LiteralPath $noticesDirectory -Force | Where-Object {
    $_.PSIsContainer -or $_.Name -notin $expectedNoticeNames
})
if ($unexpectedNotices.Count -gt 0) {
    throw "FFmpeg notice directory contains unmanaged entries: $($unexpectedNotices.Name -join ', ')"
}

if (-not $Force -and ($installedFiles | Where-Object { -not (Test-ExpectedFile $_.Path $_.Hash) }).Count -eq 0) {
    Write-Output "Verified FFmpeg $($manifest.version) sidecars and notices for $targetTriple."
    exit 0
}

$operationId = [Guid]::NewGuid().ToString('N')
$temporaryRoot = [IO.Path]::GetTempPath()
$archivePath = Join-Path $temporaryRoot "vidmetry-ffmpeg-$operationId.zip"
$extractDirectory = Join-Path $temporaryRoot "vidmetry-ffmpeg-$operationId"
$resolvedTemp = [IO.Path]::GetFullPath($temporaryRoot).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$resolvedExtract = [IO.Path]::GetFullPath($extractDirectory)
if (-not $resolvedExtract.StartsWith($resolvedTemp, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to use an extraction directory outside the temporary directory: $resolvedExtract"
}

$stagedFiles = @()
try {
    Write-Output "Downloading pinned FFmpeg $($manifest.version) essentials build..."
    & curl.exe --fail --location --retry 5 --retry-all-errors --connect-timeout 15 --max-time 600 --silent --show-error --output $archivePath $manifest.archive.url
    if ($LASTEXITCODE -ne 0) {
        throw "FFmpeg download failed with curl exit code $LASTEXITCODE."
    }
    $actualArchiveHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualArchiveHash -ne $manifest.archive.sha256) {
        throw "FFmpeg archive checksum mismatch. Expected $($manifest.archive.sha256) but received $actualArchiveHash."
    }

    New-Item -ItemType Directory -Path $extractDirectory | Out-Null
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDirectory
    $sources = @(
        @{ Path = Resolve-ArchiveFile $extractDirectory $target.ffmpeg.archivePath; Hash = $target.ffmpeg.sha256; Target = $ffmpegTarget },
        @{ Path = Resolve-ArchiveFile $extractDirectory $target.ffprobe.archivePath; Hash = $target.ffprobe.sha256; Target = $ffprobeTarget },
        @{ Path = Resolve-ArchiveFile $extractDirectory $manifest.notices.license.archivePath; Hash = $manifest.notices.license.sha256; Target = $licenseTarget },
        @{ Path = Resolve-ArchiveFile $extractDirectory $manifest.notices.buildInfo.archivePath; Hash = $manifest.notices.buildInfo.sha256; Target = $buildInfoTarget }
    )
    foreach ($source in $sources) {
        $actualHash = (Get-FileHash -LiteralPath $source.Path -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actualHash -ne $source.Hash) {
            throw "FFmpeg archive entry checksum mismatch for $($source.Path). Expected $($source.Hash) but received $actualHash."
        }
        $staged = "$($source.Target).$operationId.tmp"
        Copy-Item -LiteralPath $source.Path -Destination $staged
        $stagedFiles += @{ Path = $staged; Target = $source.Target }
    }
    foreach ($staged in $stagedFiles) {
        Move-Item -LiteralPath $staged.Path -Destination $staged.Target -Force
    }

    Write-Output "Installed and verified FFmpeg $($manifest.version) sidecars for $targetTriple."
    & $ffmpegTarget -version | Select-Object -First 1
} finally {
    foreach ($staged in $stagedFiles) {
        if (Test-Path -LiteralPath $staged.Path) {
            Remove-Item -LiteralPath $staged.Path -Force
        }
    }
    if (Test-Path -LiteralPath $archivePath) {
        Remove-Item -LiteralPath $archivePath -Force
    }
    if (Test-Path -LiteralPath $resolvedExtract) {
        Remove-Item -LiteralPath $resolvedExtract -Recurse -Force
    }
}
