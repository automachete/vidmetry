[CmdletBinding()]
param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$binariesDirectory = Join-Path $projectRoot 'src-tauri\binaries'
$archiveUrl = 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip'
$checksumUrl = "$archiveUrl.sha256"
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$rustcPath = Join-Path $cargoBin 'rustc.exe'

if (-not (Test-Path -LiteralPath $rustcPath)) {
    throw 'rustc was not found. Install Rust with rustup before preparing FFmpeg sidecars.'
}

$targetTriple = (& $rustcPath --print host-tuple).Trim()
if ([string]::IsNullOrWhiteSpace($targetTriple)) {
    throw 'Unable to determine the Rust host target triple.'
}

New-Item -ItemType Directory -Path $binariesDirectory -Force | Out-Null
$ffmpegTarget = Join-Path $binariesDirectory "ffmpeg-$targetTriple.exe"
$ffprobeTarget = Join-Path $binariesDirectory "ffprobe-$targetTriple.exe"

if (-not $Force -and (Test-Path -LiteralPath $ffmpegTarget) -and (Test-Path -LiteralPath $ffprobeTarget)) {
    Write-Output "FFmpeg sidecars already exist for $targetTriple. Use -Force to refresh them."
    exit 0
}

$operationId = [Guid]::NewGuid().ToString('N')
$archivePath = Join-Path $env:TEMP "vidmetry-ffmpeg-$operationId.zip"
$extractDirectory = Join-Path $env:TEMP "vidmetry-ffmpeg-$operationId"
$resolvedTemp = [IO.Path]::GetFullPath($env:TEMP).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$resolvedExtract = [IO.Path]::GetFullPath($extractDirectory)

if (-not $resolvedExtract.StartsWith($resolvedTemp, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to use an extraction directory outside the temporary directory: $resolvedExtract"
}

try {
    Write-Output 'Downloading FFmpeg essentials build...'
    & curl.exe --fail --location --retry 3 --output $archivePath $archiveUrl
    if ($LASTEXITCODE -ne 0) {
        throw "FFmpeg download failed with curl exit code $LASTEXITCODE."
    }
    $expectedHash = ((Invoke-WebRequest -Uri $checksumUrl).Content -split '\s+')[0].Trim().ToLowerInvariant()
    $actualHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $expectedHash) {
        throw "FFmpeg archive checksum mismatch. Expected $expectedHash but received $actualHash."
    }

    New-Item -ItemType Directory -Path $extractDirectory | Out-Null
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDirectory
    $ffmpegSource = Get-ChildItem -LiteralPath $extractDirectory -Recurse -Filter 'ffmpeg.exe' | Select-Object -First 1
    $ffprobeSource = Get-ChildItem -LiteralPath $extractDirectory -Recurse -Filter 'ffprobe.exe' | Select-Object -First 1
    if ($null -eq $ffmpegSource -or $null -eq $ffprobeSource) {
        throw 'The downloaded archive did not contain ffmpeg.exe and ffprobe.exe.'
    }

    Copy-Item -LiteralPath $ffmpegSource.FullName -Destination $ffmpegTarget -Force
    Copy-Item -LiteralPath $ffprobeSource.FullName -Destination $ffprobeTarget -Force
    Write-Output "Installed FFmpeg sidecars for $targetTriple."
    & $ffmpegTarget -version | Select-Object -First 1
} finally {
    if (Test-Path -LiteralPath $archivePath) {
        Remove-Item -LiteralPath $archivePath -Force
    }
    if (Test-Path -LiteralPath $resolvedExtract) {
        Remove-Item -LiteralPath $resolvedExtract -Recurse -Force
    }
}
