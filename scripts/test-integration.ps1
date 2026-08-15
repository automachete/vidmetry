[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$binaryRoot = Join-Path $projectRoot 'src-tauri\binaries'
$ffmpeg = Join-Path $binaryRoot 'ffmpeg-x86_64-pc-windows-msvc.exe'
$ffprobe = Join-Path $binaryRoot 'ffprobe-x86_64-pc-windows-msvc.exe'
$resultRoot = Join-Path $projectRoot 'test-results\integration'

if (-not (Test-Path -LiteralPath $ffmpeg) -or -not (Test-Path -LiteralPath $ffprobe)) {
    throw 'FFmpeg sidecars are missing. Run scripts/setup-ffmpeg.ps1 first.'
}

New-Item -ItemType Directory -Path $resultRoot -Force | Out-Null
$source = Join-Path $resultRoot 'source.mp4'
$compatible = Join-Path $resultRoot 'compatible.mp4'
$lossless = Join-Path $resultRoot 'lossless.mkv'
$metadata = Join-Path $resultRoot 'metadata.mp4'
$sourceFrames = Join-Path $resultRoot 'source-crop.framemd5'
$losslessFrames = Join-Path $resultRoot 'lossless.framemd5'

& $ffmpeg -hide_banner -loglevel error -y `
    -f lavfi -i 'testsrc2=size=1280x720:rate=30' `
    -f lavfi -i 'sine=frequency=880:sample_rate=48000' `
    -t 4 -c:v libx264 -pix_fmt yuv420p -c:a aac -movflags +faststart $source
if ($LASTEXITCODE -ne 0) { throw 'Unable to generate the integration fixture.' }
$sourceHashBefore = (Get-FileHash -Algorithm SHA256 -LiteralPath $source).Hash

& $ffmpeg -hide_banner -loglevel error -nostdin -y -i $source `
    -map '0:v:0' -map '0:a:0?' -vf 'crop=w=640:h=360:x=100:y=100,setsar=1' `
    -c:v libx264 -preset medium -crf 17 -pix_fmt yuv420p -fps_mode passthrough `
    -metadata:s:v:0 'rotate=0' -c:a copy -movflags +faststart $compatible
if ($LASTEXITCODE -ne 0) { throw 'Compatible export failed.' }

& $ffmpeg -hide_banner -loglevel error -nostdin -y -i $source `
    -map '0:v:0' -map '0:a?' -map '0:s?' -vf 'crop=w=640:h=360:x=100:y=100,setsar=1' `
    -c:v ffv1 -level 3 -coder 1 -context 1 -slicecrc 1 -c:a copy -c:s copy `
    -fps_mode passthrough -metadata:s:v:0 'rotate=0' $lossless
if ($LASTEXITCODE -ne 0) { throw 'Lossless export failed.' }

& $ffmpeg -hide_banner -loglevel error -nostdin -y -noautorotate -i $source `
    -map 0 -c copy -bsf:v:0 'h264_metadata=crop_left=100:crop_right=540:crop_top=100:crop_bottom=260' $metadata
if ($LASTEXITCODE -ne 0) { throw 'Metadata-only export failed.' }

function Get-VideoDescriptor([string]$Path) {
    $json = & $ffprobe -v error -select_streams 'v:0' `
        -show_entries 'stream=codec_name,width,height,pix_fmt' -of json $Path
    if ($LASTEXITCODE -ne 0) { throw "ffprobe failed for $Path" }
    return ($json | ConvertFrom-Json).streams[0]
}

$compatibleInfo = Get-VideoDescriptor $compatible
$losslessInfo = Get-VideoDescriptor $lossless
$metadataInfo = Get-VideoDescriptor $metadata

if ($compatibleInfo.codec_name -ne 'h264' -or $compatibleInfo.width -ne 640 -or $compatibleInfo.height -ne 360) {
    throw 'Compatible output descriptor does not match the selected crop.'
}
if ($losslessInfo.codec_name -ne 'ffv1' -or $losslessInfo.width -ne 640 -or $losslessInfo.height -ne 360) {
    throw 'Lossless output descriptor does not match the selected crop.'
}
if ($metadataInfo.codec_name -ne 'h264' -or $metadataInfo.width -ne 640 -or $metadataInfo.height -ne 360) {
    throw 'Metadata-only output did not expose the intended display crop.'
}

& $ffmpeg -hide_banner -loglevel error -y -i $source -an `
    -vf 'crop=w=640:h=360:x=100:y=100,setsar=1' -f framemd5 $sourceFrames
if ($LASTEXITCODE -ne 0) { throw 'Unable to fingerprint source crop frames.' }
& $ffmpeg -hide_banner -loglevel error -y -i $lossless -an -f framemd5 $losslessFrames
if ($LASTEXITCODE -ne 0) { throw 'Unable to fingerprint lossless output frames.' }
if ((Get-FileHash -Algorithm SHA256 -LiteralPath $sourceFrames).Hash -ne `
    (Get-FileHash -Algorithm SHA256 -LiteralPath $losslessFrames).Hash) {
    throw 'Lossless output pixels differ from the decoded source crop.'
}

$sourceHashAfter = (Get-FileHash -Algorithm SHA256 -LiteralPath $source).Hash
if ($sourceHashBefore -ne $sourceHashAfter) {
    throw 'The source fixture changed during export tests.'
}

[pscustomobject]@{
    Compatible = "h264 $($compatibleInfo.width)x$($compatibleInfo.height)"
    Lossless = "ffv1 $($losslessInfo.width)x$($losslessInfo.height)"
    MetadataOnly = "h264 copy $($metadataInfo.width)x$($metadataInfo.height)"
    SourceUnchanged = $true
    LosslessPixelsMatch = $true
}
