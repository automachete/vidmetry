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
    if ($Value -cnotmatch '^[0-9a-f]{64}$') {
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

function Test-ExpectedText([string]$Path, [string]$Expected) {
    return (Test-Path -LiteralPath $Path -PathType Leaf) -and
        ([IO.File]::ReadAllText($Path) -ceq $Expected)
}

function Write-Utf8NoBom([string]$Path, [string]$Content) {
    [IO.File]::WriteAllText($Path, $Content, [Text.UTF8Encoding]::new($false))
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

function Get-EngineRuntime([string]$FfmpegPath, [psobject]$Engine) {
    $versionLines = @(& $FfmpegPath -version 2>&1 | ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0) {
        throw "FFmpeg version inspection failed with exit code $LASTEXITCODE."
    }
    $versionText = ($versionLines -join "`n").Trim()
    $firstLine = ($versionText -split "`n", 2)[0]
    if (-not $firstLine.StartsWith($Engine.versionPrefix, [StringComparison]::Ordinal)) {
        throw "FFmpeg version mismatch. Expected prefix '$($Engine.versionPrefix)' but received '$firstLine'."
    }
    $tokens = @($versionText -split '\s+')
    foreach ($flag in $Engine.requiredConfigurationFlags) {
        if ($flag -cnotin $tokens) {
            throw "FFmpeg is missing required configuration flag $flag."
        }
    }
    foreach ($flag in $Engine.forbiddenConfigurationFlags) {
        if ($flag -cin $tokens) {
            throw "FFmpeg contains forbidden, non-redistributable configuration flag $flag."
        }
    }

    $encoderLines = @(& $FfmpegPath -hide_banner -encoders 2>&1 | ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0) {
        throw "FFmpeg encoder inspection failed with exit code $LASTEXITCODE."
    }
    $encoderText = $encoderLines -join "`n"
    foreach ($encoder in $Engine.requiredEncoders) {
        $pattern = "(?m)^\s*[A-Z.]{6}\s+$([regex]::Escape($encoder))\s+"
        if ($encoderText -cnotmatch $pattern) {
            throw "FFmpeg is missing required encoder $encoder."
        }
    }

    return $versionText
}

function Get-NoticeContents(
    [psobject]$Manifest,
    [string]$VersionText,
    [string]$ReleaseTag,
    [string]$SourceUrl
) {
    $buildInfo = @"
Vidmetry FFmpeg sidecar build information

Engine identifier: $($Manifest.engine.id)
License: $($Manifest.engine.license)
Build variant: $($Manifest.engine.variant)
Binary provider: $($Manifest.archive.provider)
Binary release: https://github.com/$($Manifest.archive.provider)/releases/tag/$($Manifest.archive.releaseTag)
Binary archive: $($Manifest.archive.url)
Binary archive SHA-256: $($Manifest.archive.sha256)
Build scripts: $($Manifest.correspondingSource.buildRepository) at $($Manifest.correspondingSource.buildCommit)
FFmpeg source: $($Manifest.correspondingSource.ffmpegRepository) at $($Manifest.correspondingSource.ffmpegCommit)
Complete corresponding source for Vidmetry ${ReleaseTag}: $SourceUrl

Runtime report:
$VersionText
"@.Replace("`r`n", "`n").TrimEnd() + "`n"

    $sourceNotice = @"
FFmpeg complete corresponding source

The FFmpeg and ffprobe executables in this Vidmetry distribution are licensed under $($Manifest.engine.license).
Equivalent access to their machine-readable Complete Corresponding Source is offered next to the installers in the same official GitHub Release, at no charge:

$SourceUrl

Source archive name: $($Manifest.correspondingSource.archiveName)
The adjacent .sha256 asset authenticates the source archive. The archive contains the exact FFmpeg source, dependency source archives selected by the pinned Windows GPL build graph, and the build scripts and patches at the pinned build commit.

Vidmetry invokes FFmpeg and ffprobe as separate command-line programs. Vidmetry's MIT license does not replace, restrict, or modify the rights granted for those programs by the GNU GPL.
"@.Replace("`r`n", "`n").TrimEnd() + "`n"

    return @{
        BuildInfo = $buildInfo
        SourceNotice = $sourceNotice
    }
}

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "FFmpeg sidecar manifest was not found: $manifestPath"
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
Assert-ExactProperties $manifest @('schemaVersion', 'engine', 'archive', 'targets', 'notices', 'correspondingSource') 'manifest'
Assert-ExactProperties $manifest.engine @('id', 'versionPrefix', 'license', 'variant', 'requiredConfigurationFlags', 'forbiddenConfigurationFlags', 'requiredEncoders') 'engine'
Assert-ExactProperties $manifest.archive @('provider', 'releaseTag', 'url', 'sha256', 'rootDirectory') 'archive'
Assert-ExactProperties $manifest.notices @('license', 'buildInfoFileName', 'correspondingSourceFileName') 'notices'
Assert-ExactProperties $manifest.notices.license @('archivePath', 'sha256', 'fileName') 'notices.license'
Assert-ExactProperties $manifest.correspondingSource @('archiveName', 'buildRepository', 'buildCommit', 'ffmpegRepository', 'ffmpegCommit', 'officialReleaseBaseUrl') 'correspondingSource'

if ($manifest.schemaVersion -ne 1) { throw 'FFmpeg manifest schemaVersion must be 1.' }
if ($manifest.engine.license -cne 'GPL-3.0-or-later' -or $manifest.engine.variant -cne 'win64-gpl') {
    throw 'FFmpeg must use the pinned GPL Windows build contract.'
}
if ($manifest.archive.provider -cne 'BtbN/FFmpeg-Builds') {
    throw 'FFmpeg archive provider is not the approved reproducible-build project.'
}
if ($manifest.archive.releaseTag -notmatch '^autobuild-\d{4}-\d{2}-\d{2}-\d{2}-\d{2}$') {
    throw 'FFmpeg archive releaseTag must be an immutable dated Auto-Build tag.'
}
$expectedArchiveUrl = "https://github.com/$($manifest.archive.provider)/releases/download/$($manifest.archive.releaseTag)/$($manifest.archive.rootDirectory).zip"
if ($manifest.archive.url -cne $expectedArchiveUrl -or $manifest.archive.url -match '/latest/') {
    throw 'FFmpeg archive URL must identify the immutable dated GitHub Release asset.'
}
Assert-Sha256 $manifest.archive.sha256 'archive.sha256'
Assert-Sha256 $manifest.notices.license.sha256 'notices.license.sha256'
if ($manifest.notices.license.fileName -cne 'FFMPEG_LICENSE.txt' -or
    $manifest.notices.buildInfoFileName -cne 'FFMPEG_BUILD_INFO.txt' -or
    $manifest.notices.correspondingSourceFileName -cne 'FFMPEG_CORRESPONDING_SOURCE.txt') {
    throw 'FFmpeg notice output names must match the bundled notice contract.'
}
if ($manifest.correspondingSource.archiveName -cnotmatch '^vidmetry-ffmpeg-[A-Za-z0-9.-]+-corresponding-source\.tar\.xz$') {
    throw 'Corresponding-source archive name is invalid.'
}
if ($manifest.correspondingSource.buildRepository -cne 'https://github.com/BtbN/FFmpeg-Builds.git' -or
    $manifest.correspondingSource.ffmpegRepository -cne 'https://github.com/FFmpeg/FFmpeg.git' -or
    $manifest.correspondingSource.officialReleaseBaseUrl -cne 'https://github.com/automachete/vidmetry/releases/download') {
    throw 'Corresponding-source repositories or official release location are not approved.'
}
if ($manifest.correspondingSource.buildCommit -cnotmatch '^[0-9a-f]{40}$' -or
    $manifest.correspondingSource.ffmpegCommit -cnotmatch '^[0-9a-f]{40}$') {
    throw 'Corresponding-source commits must be full lowercase Git commit SHAs.'
}
if (@($manifest.engine.requiredConfigurationFlags).Count -lt 4 -or
    '--enable-nonfree' -cnotin @($manifest.engine.forbiddenConfigurationFlags) -or
    @($manifest.engine.requiredEncoders).Count -lt 4) {
    throw 'FFmpeg capability requirements are incomplete.'
}

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

$packageVersion = (Get-Content -LiteralPath (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json).version
$releaseTag = "v$packageVersion"
$sourceUrl = "$($manifest.correspondingSource.officialReleaseBaseUrl)/$releaseTag/$($manifest.correspondingSource.archiveName)"

New-Item -ItemType Directory -Path $binariesDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $noticesDirectory -Force | Out-Null
$ffmpegTarget = Join-Path $binariesDirectory "ffmpeg-$targetTriple.exe"
$ffprobeTarget = Join-Path $binariesDirectory "ffprobe-$targetTriple.exe"
$licenseTarget = Join-Path $noticesDirectory $manifest.notices.license.fileName
$buildInfoTarget = Join-Path $noticesDirectory $manifest.notices.buildInfoFileName
$sourceNoticeTarget = Join-Path $noticesDirectory $manifest.notices.correspondingSourceFileName
$expectedNoticeNames = @(
    $manifest.notices.license.fileName,
    $manifest.notices.buildInfoFileName,
    $manifest.notices.correspondingSourceFileName
)
$unexpectedNotices = @(Get-ChildItem -LiteralPath $noticesDirectory -Force | Where-Object {
    $_.PSIsContainer -or $_.Name -notin $expectedNoticeNames
})
if ($unexpectedNotices.Count -gt 0) {
    throw "FFmpeg notice directory contains unmanaged entries: $($unexpectedNotices.Name -join ', ')"
}

$binaryFilesValid = (Test-ExpectedFile $ffmpegTarget $target.ffmpeg.sha256) -and
    (Test-ExpectedFile $ffprobeTarget $target.ffprobe.sha256) -and
    (Test-ExpectedFile $licenseTarget $manifest.notices.license.sha256)
if (-not $Force -and $binaryFilesValid) {
    $runtime = Get-EngineRuntime $ffmpegTarget $manifest.engine
    $notices = Get-NoticeContents $manifest $runtime $releaseTag $sourceUrl
    if ((Test-ExpectedText $buildInfoTarget $notices.BuildInfo) -and
        (Test-ExpectedText $sourceNoticeTarget $notices.SourceNotice)) {
        Write-Output "Verified FFmpeg $($manifest.engine.id) sidecars, capabilities, license, and source notice for $targetTriple."
        exit 0
    }
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
    Write-Output "Downloading pinned FFmpeg $($manifest.engine.id) GPL build..."
    & curl.exe --fail --location --retry 5 --retry-all-errors --connect-timeout 15 --max-time 900 --silent --show-error --output $archivePath $manifest.archive.url
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
        @{ Path = Resolve-ArchiveFile $extractDirectory $manifest.notices.license.archivePath; Hash = $manifest.notices.license.sha256; Target = $licenseTarget }
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

    $runtime = Get-EngineRuntime $ffmpegTarget $manifest.engine
    $notices = Get-NoticeContents $manifest $runtime $releaseTag $sourceUrl
    Write-Utf8NoBom $buildInfoTarget $notices.BuildInfo
    Write-Utf8NoBom $sourceNoticeTarget $notices.SourceNotice

    Write-Output "Installed and verified FFmpeg $($manifest.engine.id) sidecars for $targetTriple."
    Write-Output ($runtime -split "`n", 2)[0]
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
