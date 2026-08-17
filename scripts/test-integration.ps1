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
$custom = Join-Path $resultRoot 'custom-hevc.mp4'
$lossless = Join-Path $resultRoot 'lossless.mkv'
$metadata = Join-Path $resultRoot 'metadata.mp4'
$trimmed = Join-Path $resultRoot 'time-trimmed.mp4'
$inPlaceSource = Join-Path $resultRoot 'in-place-source.mp4'
$inPlaceTemporary = Join-Path $resultRoot 'in-place-source.vidmetry-test.tmp.mp4'
$sourceFrames = Join-Path $resultRoot 'source-crop.framemd5'
$losslessFrames = Join-Path $resultRoot 'lossless.framemd5'

& $ffmpeg -hide_banner -loglevel error -y `
    -f lavfi -i 'testsrc2=size=1280x720:rate=30' `
    -f lavfi -i 'sine=frequency=880:sample_rate=48000' `
    -t 4 -vf 'setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=limited' `
    -c:v libx264 -pix_fmt yuv420p `
    -c:a aac -movflags +faststart $source
if ($LASTEXITCODE -ne 0) { throw 'Unable to generate the integration fixture.' }
$sourceHashBefore = (Get-FileHash -Algorithm SHA256 -LiteralPath $source).Hash

function Invoke-EncoderFallback(
    [string]$Output,
    [string[]]$BeforeEncoder,
    [object[]]$Attempts,
    [string[]]$AfterEncoder
) {
    foreach ($attempt in $Attempts) {
        $encoderArguments = [string[]]$attempt.Arguments
        & $ffmpeg @BeforeEncoder @encoderArguments @AfterEncoder $Output
        if ($LASTEXITCODE -eq 0) { return [string]$attempt.Name }
        [System.IO.File]::Delete($Output)
    }
    throw "Every compatible encoder failed for $Output"
}

$compatibleEncoder = Invoke-EncoderFallback `
    -Output $compatible `
    -BeforeEncoder @(
        '-hide_banner', '-loglevel', 'error', '-nostdin', '-y', '-i', $source,
        '-map', '0:v:0', '-map', '0:a:0?',
        '-vf', 'crop=w=640:h=360:x=100:y=100,setsar=1,setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=limited'
    ) `
    -Attempts @(
        @{ Name = 'h264_nvenc'; Arguments = @('-c:v', 'h264_nvenc', '-preset', 'p4', '-tune', 'hq', '-rc', 'vbr', '-cq', '17', '-b:v', '0') },
        @{ Name = 'h264_qsv'; Arguments = @('-c:v', 'h264_qsv', '-preset', 'medium', '-global_quality', '17') },
        @{ Name = 'h264_amf'; Arguments = @('-c:v', 'h264_amf', '-usage', 'transcoding', '-quality', 'balanced', '-rc', 'qvbr', '-qvbr_quality_level', '17') },
        @{ Name = 'libx264'; Arguments = @('-c:v', 'libx264', '-preset', 'medium', '-crf', '17') }
    ) `
    -AfterEncoder @(
        '-pix_fmt', 'yuv420p', '-fps_mode', 'passthrough',
        '-metadata:s:v:0', 'rotate=0', '-c:a', 'copy', '-movflags', '+faststart'
    )

$customEncoder = Invoke-EncoderFallback `
    -Output $custom `
    -BeforeEncoder @(
        '-hide_banner', '-loglevel', 'error', '-nostdin', '-y', '-i', $source,
        '-map', '0:v:0', '-map', '0:a:0?',
        '-vf', 'crop=w=640:h=360:x=100:y=100,setsar=1,setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=limited'
    ) `
    -Attempts @(
        @{ Name = 'hevc_nvenc'; Arguments = @('-c:v', 'hevc_nvenc', '-preset', 'p4', '-tune', 'hq', '-rc', 'vbr', '-cq', '23', '-b:v', '0') },
        @{ Name = 'hevc_qsv'; Arguments = @('-c:v', 'hevc_qsv', '-preset', 'fast', '-global_quality', '23') },
        @{ Name = 'hevc_amf'; Arguments = @('-c:v', 'hevc_amf', '-usage', 'transcoding', '-quality', 'balanced', '-rc', 'qvbr', '-qvbr_quality_level', '23') },
        @{ Name = 'libx265'; Arguments = @('-c:v', 'libx265', '-preset', 'fast', '-crf', '23') }
    ) `
    -AfterEncoder @(
        '-pix_fmt', 'yuv420p10le', '-r', '24', '-fps_mode', 'cfr',
        '-c:a', 'aac', '-b:a', '160k', '-map_metadata', '-1'
    )

& $ffmpeg -hide_banner -loglevel error -nostdin -y -i $source `
    -map '0:v:0' -map '0:a?' -map '0:s?' `
    -vf 'crop=w=640:h=360:x=100:y=100,setsar=1,setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=limited' `
    -c:v ffv1 -level 3 -coder 1 -context 1 -slicecrc 1 -c:a copy -c:s copy `
    -fps_mode passthrough `
    -metadata:s:v:0 'rotate=0' $lossless
if ($LASTEXITCODE -ne 0) { throw 'Lossless export failed.' }

& $ffmpeg -hide_banner -loglevel error -nostdin -y -noautorotate -i $source `
    -map 0 -c copy -bsf:v:0 'h264_metadata=crop_left=100:crop_right=540:crop_top=100:crop_bottom=260' $metadata
if ($LASTEXITCODE -ne 0) { throw 'Metadata-only export failed.' }

& $ffmpeg -hide_banner -loglevel error -nostdin -y -i $source `
    -map '0:v:0' -map '0:a:0?' `
    -vf 'crop=w=640:h=360:x=100:y=100,setsar=1,trim=start_frame=30:end_frame=90,setpts=PTS-STARTPTS' `
    -af 'atrim=start=1:end=3,asetpts=PTS-STARTPTS' `
    -c:v libx264 -preset medium -crf 17 -pix_fmt yuv420p -fps_mode passthrough `
    -c:a aac -b:a 192k -t 2 $trimmed
if ($LASTEXITCODE -ne 0) { throw 'Frame-accurate time trim export failed.' }

function Get-VideoDescriptor([string]$Path) {
    $json = & $ffprobe -v error -select_streams 'v:0' `
        -show_entries 'stream=codec_name,width,height,pix_fmt,nb_frames,duration,color_primaries,color_transfer,color_space,color_range' -of json $Path
    if ($LASTEXITCODE -ne 0) { throw "ffprobe failed for $Path" }
    return ($json | ConvertFrom-Json).streams[0]
}

$compatibleInfo = Get-VideoDescriptor $compatible
$customInfo = Get-VideoDescriptor $custom
$losslessInfo = Get-VideoDescriptor $lossless
$metadataInfo = Get-VideoDescriptor $metadata
$trimmedInfo = Get-VideoDescriptor $trimmed

if ($compatibleInfo.codec_name -ne 'h264' -or $compatibleInfo.width -ne 640 -or $compatibleInfo.height -ne 360) {
    throw 'Compatible output descriptor does not match the selected crop.'
}
if ($customInfo.codec_name -ne 'hevc' -or $customInfo.pix_fmt -ne 'yuv420p10le' -or $customInfo.width -ne 640 -or $customInfo.height -ne 360) {
    throw 'Custom output did not apply the configured codec or pixel format.'
}
if ($losslessInfo.codec_name -ne 'ffv1' -or $losslessInfo.width -ne 640 -or $losslessInfo.height -ne 360) {
    throw 'Lossless output descriptor does not match the selected crop.'
}
foreach ($descriptor in @($compatibleInfo, $customInfo, $losslessInfo)) {
    if ($descriptor.color_primaries -ne 'bt709' -or $descriptor.color_transfer -ne 'bt709' -or `
        $descriptor.color_space -ne 'bt709' -or $descriptor.color_range -ne 'tv') {
        throw 'A re-encoded output did not preserve the source color description.'
    }
}
if ($metadataInfo.codec_name -ne 'h264' -or $metadataInfo.width -ne 640 -or $metadataInfo.height -ne 360) {
    throw 'Metadata-only output did not expose the intended display crop.'
}
if ([int]$trimmedInfo.nb_frames -ne 60 -or [Math]::Abs([double]$trimmedInfo.duration - 2.0) -gt 0.05) {
    throw 'Time-trimmed output is not the selected 60-frame, two-second range.'
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

Copy-Item -LiteralPath $source -Destination $inPlaceSource -Force
$inPlaceHashBefore = (Get-FileHash -Algorithm SHA256 -LiteralPath $inPlaceSource).Hash
& $ffmpeg -hide_banner -loglevel error -nostdin -y -i $inPlaceSource `
    -map '0:v:0' -map '0:a:0?' -vf 'crop=w=640:h=360:x=100:y=100,setsar=1' `
    -c:v libx264 -preset fast -crf 20 -pix_fmt yuv420p -c:a copy $inPlaceTemporary
if ($LASTEXITCODE -ne 0) { throw 'In-place staging export failed.' }
[System.IO.File]::Move($inPlaceTemporary, $inPlaceSource, $true)
$inPlaceInfo = Get-VideoDescriptor $inPlaceSource
$inPlaceHashAfter = (Get-FileHash -Algorithm SHA256 -LiteralPath $inPlaceSource).Hash
if ($inPlaceHashBefore -eq $inPlaceHashAfter -or $inPlaceInfo.width -ne 640 -or $inPlaceInfo.height -ne 360) {
    throw 'In-place replacement did not atomically replace the source with the cropped result.'
}

$sourceHashAfter = (Get-FileHash -Algorithm SHA256 -LiteralPath $source).Hash
if ($sourceHashBefore -ne $sourceHashAfter) {
    throw 'The source fixture changed during export tests.'
}

[pscustomobject]@{
    Compatible = "h264 via $compatibleEncoder $($compatibleInfo.width)x$($compatibleInfo.height)"
    Custom = "hevc via $customEncoder/$($customInfo.pix_fmt) $($customInfo.width)x$($customInfo.height)"
    Lossless = "ffv1 $($losslessInfo.width)x$($losslessInfo.height)"
    MetadataOnly = "h264 copy $($metadataInfo.width)x$($metadataInfo.height)"
    TimeTrim = "$($trimmedInfo.nb_frames) frames / $($trimmedInfo.duration)s"
    InPlaceReplacement = $true
    SourceUnchanged = $true
    LosslessPixelsMatch = $true
}
