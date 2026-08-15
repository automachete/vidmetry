use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_shell::{
    ShellExt,
    process::{CommandChild, CommandEvent},
};
use uuid::Uuid;

use crate::{
    ffmpeg::{self, MediaError},
    media::MediaDescriptor,
};

const PROGRESS_EVENT: &str = "export-progress";
const COMPLETED_EVENT: &str = "export-complete";
const FAILED_EVENT: &str = "export-error";

#[derive(Default)]
pub struct ExportState {
    jobs: Mutex<HashMap<String, CommandChild>>,
    cancelled: Mutex<HashSet<String>>,
}

impl Drop for ExportState {
    fn drop(&mut self) {
        if let Ok(jobs) = self.jobs.get_mut() {
            for (_, child) in jobs.drain() {
                let _ = child.kill();
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExportProfile {
    Compatible,
    Lossless,
    Metadata,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VideoCodec {
    H264,
    H265,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EncoderPreset {
    Ultrafast,
    Superfast,
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    Veryslow,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PixelFormat {
    Source,
    Yuv420p,
    Yuv420p10le,
    Yuv422p,
    Yuv422p10le,
    Yuv444p,
    Yuv444p10le,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AudioMode {
    Auto,
    Copy,
    Aac,
    Flac,
    Pcm,
    None,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FrameRateMode {
    Passthrough,
    Constant,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExportSettings {
    pub profile: ExportProfile,
    pub video_codec: VideoCodec,
    pub crf: u8,
    pub preset: EncoderPreset,
    pub pixel_format: PixelFormat,
    pub audio_mode: AudioMode,
    pub audio_bitrate_kbps: u16,
    pub frame_rate_mode: FrameRateMode,
    pub constant_frame_rate: f64,
    pub fast_start: bool,
    pub preserve_metadata: bool,
    pub copy_subtitles: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            profile: ExportProfile::Compatible,
            video_codec: VideoCodec::H264,
            crf: 17,
            preset: EncoderPreset::Medium,
            pixel_format: PixelFormat::Yuv420p,
            audio_mode: AudioMode::Auto,
            audio_bitrate_kbps: 192,
            frame_rate_mode: FrameRateMode::Passthrough,
            constant_frame_rate: 30.0,
            fast_start: true,
            preserve_metadata: true,
            copy_subtitles: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub source_path: String,
    pub output_path: String,
    pub crop: CropRect,
    #[serde(default)]
    pub settings: ExportSettings,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub in_place: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportProgress {
    job_id: String,
    fraction: f64,
    out_time_seconds: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportCompleted {
    job_id: String,
    output_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportFailed {
    job_id: String,
    message: String,
    cancelled: bool,
}

pub async fn start(
    app: AppHandle,
    state: State<'_, ExportState>,
    request: ExportRequest,
) -> Result<String, String> {
    let source = ffmpeg::canonical_source(&request.source_path).map_err(error_text)?;
    let output = validated_output(
        &source,
        &request.output_path,
        request.settings.profile,
        request.overwrite,
        request.in_place,
    )?;
    let media = ffmpeg::probe(&app, &source).await.map_err(error_text)?;
    validate_crop(request.crop, &media)?;
    validate_export_settings(&request.settings, &media)?;

    let job_id = Uuid::new_v4().to_string();
    let temporary = temporary_output(&output, &job_id)?;
    let args = build_export_args(&source, &temporary, request.crop, &request.settings, &media);
    let (mut receiver, child) = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|error| format!("FFmpegを準備できませんでした: {error}"))?
        .args(args)
        .spawn()
        .map_err(|error| format!("FFmpegを開始できませんでした: {error}"))?;

    state
        .jobs
        .lock()
        .map_err(|_| "書き出し状態を更新できませんでした。".to_owned())?
        .insert(job_id.clone(), child);

    let task_app = app.clone();
    let task_job_id = job_id.clone();
    let duration = media.duration_seconds;
    let overwrite = request.overwrite;
    tauri::async_runtime::spawn(async move {
        let mut diagnostics = Vec::new();
        while let Some(event) = receiver.recv().await {
            match event {
                CommandEvent::Stdout(bytes) => {
                    if let Some(seconds) = parse_progress_time(&bytes) {
                        let fraction = if duration > 0.0 {
                            (seconds / duration).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let _ = task_app.emit(
                            PROGRESS_EVENT,
                            ExportProgress {
                                job_id: task_job_id.clone(),
                                fraction,
                                out_time_seconds: seconds,
                            },
                        );
                    }
                }
                CommandEvent::Stderr(bytes) => {
                    let line = String::from_utf8_lossy(&bytes).trim().to_owned();
                    if !line.is_empty() {
                        diagnostics.push(line);
                        if diagnostics.len() > 12 {
                            diagnostics.remove(0);
                        }
                    }
                }
                CommandEvent::Error(message) => diagnostics.push(message),
                CommandEvent::Terminated(status) => {
                    remove_job(&task_app, &task_job_id);
                    let cancelled = take_cancelled(&task_app, &task_job_id);
                    if status.code == Some(0) && !cancelled {
                        match commit_output(&temporary, &output, overwrite) {
                            Ok(()) => {
                                let _ = task_app.emit(
                                    COMPLETED_EVENT,
                                    ExportCompleted {
                                        job_id: task_job_id.clone(),
                                        output_path: output.to_string_lossy().into_owned(),
                                    },
                                );
                            }
                            Err(message) => {
                                let _ = fs::remove_file(&temporary);
                                emit_failure(&task_app, &task_job_id, message, false);
                            }
                        }
                    } else {
                        let _ = fs::remove_file(&temporary);
                        let message = if cancelled {
                            "書き出しをキャンセルしました。".to_owned()
                        } else if diagnostics.is_empty() {
                            format!("FFmpegが終了コード{:?}で停止しました。", status.code)
                        } else {
                            diagnostics.join(" ")
                        };
                        emit_failure(&task_app, &task_job_id, message, cancelled);
                    }
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(job_id)
}

pub fn cancel(state: State<'_, ExportState>, job_id: String) -> Result<(), String> {
    let child = state
        .jobs
        .lock()
        .map_err(|_| "書き出し状態を取得できませんでした。".to_owned())?
        .remove(&job_id);
    let Some(child) = child else {
        return Ok(());
    };
    state
        .cancelled
        .lock()
        .map_err(|_| "キャンセル状態を更新できませんでした。".to_owned())?
        .insert(job_id);
    child
        .kill()
        .map_err(|error| format!("FFmpegを停止できませんでした: {error}"))
}

fn build_export_args(
    source: &Path,
    output: &Path,
    crop: CropRect,
    settings: &ExportSettings,
    media: &MediaDescriptor,
) -> Vec<String> {
    let mut args = vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-y".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-nostats".into(),
    ];
    if settings.profile == ExportProfile::Metadata {
        args.push("-noautorotate".into());
    }
    args.extend(["-i".into(), source.to_string_lossy().into_owned()]);

    match settings.profile {
        ExportProfile::Compatible => {
            args.extend([
                "-map".into(),
                "0:v:0".into(),
                "-map".into(),
                "0:a:0?".into(),
                "-vf".into(),
                crop_filter(crop),
                "-c:v".into(),
                video_encoder(settings.video_codec).into(),
                "-preset".into(),
                encoder_preset(settings.preset).into(),
                "-crf".into(),
                settings.crf.to_string(),
            ]);
            add_pixel_format(&mut args, settings.pixel_format);
            add_frame_rate(&mut args, settings);
            add_audio(&mut args, settings, media, true);
            add_metadata_mapping(&mut args, settings.preserve_metadata);
            args.extend(["-metadata:s:v:0".into(), "rotate=0".into()]);
            if settings.fast_start {
                args.extend(["-movflags".into(), "+faststart".into()]);
            }
        }
        ExportProfile::Lossless => {
            args.extend(["-map".into(), "0:v:0".into(), "-map".into(), "0:a?".into()]);
            if settings.copy_subtitles {
                args.extend(["-map".into(), "0:s?".into()]);
            }
            args.extend([
                "-vf".into(),
                crop_filter(crop),
                "-c:v".into(),
                "ffv1".into(),
                "-level".into(),
                "3".into(),
                "-coder".into(),
                "1".into(),
                "-context".into(),
                "1".into(),
                "-slicecrc".into(),
                "1".into(),
            ]);
            add_pixel_format(&mut args, settings.pixel_format);
            add_frame_rate(&mut args, settings);
            add_audio(&mut args, settings, media, false);
            if settings.copy_subtitles {
                args.extend(["-c:s".into(), "copy".into()]);
            }
            add_metadata_mapping(&mut args, settings.preserve_metadata);
            args.extend(["-metadata:s:v:0".into(), "rotate=0".into()]);
        }
        ExportProfile::Metadata => {
            let edges = coded_crop_edges(crop, media);
            let filter = if media.video_codec == "hevc" {
                "hevc_metadata"
            } else {
                "h264_metadata"
            };
            args.extend([
                "-map".into(),
                "0".into(),
                "-c".into(),
                "copy".into(),
                "-bsf:v:0".into(),
                format!(
                    "{filter}=crop_left={}:crop_right={}:crop_top={}:crop_bottom={}",
                    edges.left, edges.right, edges.top, edges.bottom
                ),
            ]);
        }
    }

    args.push(output.to_string_lossy().into_owned());
    args
}

fn video_encoder(codec: VideoCodec) -> &'static str {
    match codec {
        VideoCodec::H264 => "libx264",
        VideoCodec::H265 => "libx265",
    }
}

fn encoder_preset(preset: EncoderPreset) -> &'static str {
    match preset {
        EncoderPreset::Ultrafast => "ultrafast",
        EncoderPreset::Superfast => "superfast",
        EncoderPreset::Veryfast => "veryfast",
        EncoderPreset::Faster => "faster",
        EncoderPreset::Fast => "fast",
        EncoderPreset::Medium => "medium",
        EncoderPreset::Slow => "slow",
        EncoderPreset::Slower => "slower",
        EncoderPreset::Veryslow => "veryslow",
    }
}

fn pixel_format(format: PixelFormat) -> Option<&'static str> {
    match format {
        PixelFormat::Source => None,
        PixelFormat::Yuv420p => Some("yuv420p"),
        PixelFormat::Yuv420p10le => Some("yuv420p10le"),
        PixelFormat::Yuv422p => Some("yuv422p"),
        PixelFormat::Yuv422p10le => Some("yuv422p10le"),
        PixelFormat::Yuv444p => Some("yuv444p"),
        PixelFormat::Yuv444p10le => Some("yuv444p10le"),
    }
}

fn add_pixel_format(args: &mut Vec<String>, format: PixelFormat) {
    if let Some(format) = pixel_format(format) {
        args.extend(["-pix_fmt".into(), format.into()]);
    }
}

fn add_frame_rate(args: &mut Vec<String>, settings: &ExportSettings) {
    match settings.frame_rate_mode {
        FrameRateMode::Passthrough => {
            args.extend(["-fps_mode".into(), "passthrough".into()]);
        }
        FrameRateMode::Constant => {
            args.extend([
                "-r".into(),
                format_float(settings.constant_frame_rate),
                "-fps_mode".into(),
                "cfr".into(),
            ]);
        }
    }
}

fn add_audio(
    args: &mut Vec<String>,
    settings: &ExportSettings,
    media: &MediaDescriptor,
    compatible: bool,
) {
    let mode = match settings.audio_mode {
        AudioMode::Auto if compatible && media.audio_codec.as_deref() == Some("aac") => {
            AudioMode::Copy
        }
        AudioMode::Auto if compatible => AudioMode::Aac,
        AudioMode::Auto => AudioMode::Copy,
        selected => selected,
    };
    match mode {
        AudioMode::Auto => unreachable!("automatic audio mode is resolved above"),
        AudioMode::Copy => args.extend(["-c:a".into(), "copy".into()]),
        AudioMode::Aac => args.extend([
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            format!("{}k", settings.audio_bitrate_kbps),
        ]),
        AudioMode::Flac => args.extend(["-c:a".into(), "flac".into()]),
        AudioMode::Pcm => args.extend(["-c:a".into(), "pcm_s24le".into()]),
        AudioMode::None => args.push("-an".into()),
    }
}

fn add_metadata_mapping(args: &mut Vec<String>, preserve: bool) {
    args.extend([
        "-map_metadata".into(),
        if preserve { "0" } else { "-1" }.into(),
    ]);
}

fn format_float(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn crop_filter(crop: CropRect) -> String {
    format!(
        "crop=w={}:h={}:x={}:y={},setsar=1",
        crop.width, crop.height, crop.x, crop.y
    )
}

#[derive(Debug, PartialEq, Eq)]
struct CropEdges {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

fn coded_crop_edges(crop: CropRect, media: &MediaDescriptor) -> CropEdges {
    let display_left = crop.x;
    let display_right = media.display_width - crop.x - crop.width;
    let display_top = crop.y;
    let display_bottom = media.display_height - crop.y - crop.height;
    match media.rotation_degrees {
        90 => CropEdges {
            left: display_bottom,
            right: display_top,
            top: display_left,
            bottom: display_right,
        },
        180 => CropEdges {
            left: display_right,
            right: display_left,
            top: display_bottom,
            bottom: display_top,
        },
        270 => CropEdges {
            left: display_top,
            right: display_bottom,
            top: display_right,
            bottom: display_left,
        },
        _ => CropEdges {
            left: display_left,
            right: display_right,
            top: display_top,
            bottom: display_bottom,
        },
    }
}

fn validate_crop(crop: CropRect, media: &MediaDescriptor) -> Result<(), String> {
    if crop.width < 16 || crop.height < 16 {
        return Err("切り取り範囲は16×16ピクセル以上にしてください。".into());
    }
    if [crop.x, crop.y, crop.width, crop.height]
        .into_iter()
        .any(|value| value % 2 != 0)
    {
        return Err("切り取り座標とサイズは偶数にしてください。".into());
    }
    let right = crop
        .x
        .checked_add(crop.width)
        .ok_or_else(|| "切り取り範囲が不正です。".to_owned())?;
    let bottom = crop
        .y
        .checked_add(crop.height)
        .ok_or_else(|| "切り取り範囲が不正です。".to_owned())?;
    if right > media.display_width || bottom > media.display_height {
        return Err("切り取り範囲が動画フレームの外側です。".into());
    }
    Ok(())
}

fn validate_export_settings(
    settings: &ExportSettings,
    media: &MediaDescriptor,
) -> Result<(), String> {
    if settings.profile == ExportProfile::Metadata && !media.metadata_crop_supported {
        return Err("メタデータ方式はH.264またはHEVCの動画だけで使用できます。".into());
    }
    if settings.crf > 51 {
        return Err("CRFは0から51の範囲で指定してください。".into());
    }
    if !(32..=1024).contains(&settings.audio_bitrate_kbps) {
        return Err("音声ビットレートは32から1024 kbpsの範囲で指定してください。".into());
    }
    if settings.frame_rate_mode == FrameRateMode::Constant
        && (!settings.constant_frame_rate.is_finite()
            || !(1.0..=240.0).contains(&settings.constant_frame_rate))
    {
        return Err("固定フレームレートは1から240 fpsの範囲で指定してください。".into());
    }
    if settings.profile == ExportProfile::Compatible
        && matches!(settings.audio_mode, AudioMode::Flac | AudioMode::Pcm)
    {
        return Err("互換MP4ではFLACまたはPCM音声を選択できません。".into());
    }
    Ok(())
}

fn validated_output(
    source: &Path,
    requested: &str,
    profile: ExportProfile,
    overwrite: bool,
    in_place: bool,
) -> Result<PathBuf, String> {
    let output = PathBuf::from(requested);
    if !output.is_absolute() {
        return Err("保存先には絶対パスを指定してください。".into());
    }
    let parent = output
        .parent()
        .ok_or_else(|| "保存先フォルダーを特定できません。".to_owned())?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| format!("保存先フォルダーにアクセスできません: {error}"))?;
    if !canonical_parent.is_dir() {
        return Err("保存先フォルダーが存在しません。".into());
    }
    let file_name = output
        .file_name()
        .ok_or_else(|| "保存ファイル名を指定してください。".to_owned())?;
    let output = canonical_parent.join(file_name);
    let mut matches_source = false;
    if output.exists() {
        let existing = fs::canonicalize(&output)
            .map_err(|error| format!("保存先ファイルを確認できません: {error}"))?;
        matches_source = existing == source;
        if matches_source && !in_place {
            return Err("元動画を置き換えるには「保存」を使用してください。".into());
        }
        if !matches_source && !overwrite {
            return Err("保存先のファイルは既に存在します。".into());
        }
    }
    if in_place && !matches_source {
        return Err("「保存」の出力先は現在の元動画と一致する必要があります。".into());
    }
    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let valid = match profile {
        ExportProfile::Compatible => extension == "mp4",
        ExportProfile::Lossless => extension == "mkv",
        ExportProfile::Metadata => ["mp4", "m4v", "mov", "mkv"].contains(&extension.as_str()),
    };
    if !valid {
        return Err(match profile {
            ExportProfile::Compatible => "互換MP4の拡張子は.mp4にしてください。".into(),
            ExportProfile::Lossless => "可逆保存の拡張子は.mkvにしてください。".into(),
            ExportProfile::Metadata => {
                "メタデータ方式は.mp4、.m4v、.mov、.mkvへ保存できます。".into()
            }
        });
    }
    Ok(output)
}

fn temporary_output(output: &Path, job_id: &str) -> Result<PathBuf, String> {
    let parent = output
        .parent()
        .ok_or_else(|| "保存先フォルダーを特定できません。".to_owned())?;
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("vidmetry");
    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "保存ファイルの拡張子がありません。".to_owned())?;
    Ok(parent.join(format!(".{stem}.vidmetry-{job_id}.{extension}")))
}

fn parse_progress_time(bytes: &[u8]) -> Option<f64> {
    let line = String::from_utf8_lossy(bytes);
    let value = line
        .strip_prefix("out_time_us=")?
        .trim()
        .parse::<f64>()
        .ok()?;
    Some(value / 1_000_000.0)
}

fn remove_job(app: &AppHandle, job_id: &str) {
    if let Ok(mut jobs) = app.state::<ExportState>().jobs.lock() {
        jobs.remove(job_id);
    }
}

fn take_cancelled(app: &AppHandle, job_id: &str) -> bool {
    app.state::<ExportState>()
        .cancelled
        .lock()
        .map(|mut jobs| jobs.remove(job_id))
        .unwrap_or(false)
}

fn emit_failure(app: &AppHandle, job_id: &str, message: String, cancelled: bool) {
    let _ = app.emit(
        FAILED_EVENT,
        ExportFailed {
            job_id: job_id.to_owned(),
            message,
            cancelled,
        },
    );
}

fn error_text(error: MediaError) -> String {
    error.to_string()
}

#[cfg(windows)]
fn commit_output(temporary: &Path, output: &Path, overwrite: bool) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let temporary_wide = temporary
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let output_wide = output
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut flags = MOVEFILE_WRITE_THROUGH;
    if overwrite {
        flags |= MOVEFILE_REPLACE_EXISTING;
    }
    // SAFETY: Both pointers reference null-terminated UTF-16 buffers that live
    // for the duration of this call. Paths are validated before FFmpeg starts.
    let succeeded = unsafe { MoveFileExW(temporary_wide.as_ptr(), output_wide.as_ptr(), flags) };
    if succeeded == 0 {
        Err(format!(
            "完成した動画を保存先へ確定できませんでした: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn commit_output(temporary: &Path, output: &Path, overwrite: bool) -> Result<(), String> {
    if output.exists() && !overwrite {
        return Err("保存先のファイルは既に存在します。".into());
    }
    fs::rename(temporary, output)
        .map_err(|error| format!("完成した動画を保存先へ確定できませんでした: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::ColorDescriptor;

    fn media(rotation_degrees: i32) -> MediaDescriptor {
        let (display_width, display_height) = if [90, 270].contains(&rotation_degrees) {
            (1080, 1920)
        } else {
            (1920, 1080)
        };
        MediaDescriptor {
            source_path: "input.mp4".into(),
            file_name: "input.mp4".into(),
            duration_seconds: 12.0,
            coded_width: 1920,
            coded_height: 1080,
            display_width,
            display_height,
            rotation_degrees,
            sample_aspect_ratio: "1:1".into(),
            frame_rate: "30/1".into(),
            video_codec: "h264".into(),
            pixel_format: "yuv420p".into(),
            bit_depth: Some(8),
            has_audio: true,
            audio_codec: Some("aac".into()),
            color: ColorDescriptor::default(),
            metadata_crop_supported: true,
        }
    }

    #[test]
    fn validates_even_in_bounds_crop() {
        let descriptor = media(0);
        assert!(
            validate_crop(
                CropRect {
                    x: 100,
                    y: 80,
                    width: 800,
                    height: 600
                },
                &descriptor
            )
            .is_ok()
        );
        assert!(
            validate_crop(
                CropRect {
                    x: 101,
                    y: 80,
                    width: 800,
                    height: 600
                },
                &descriptor
            )
            .is_err()
        );
        assert!(
            validate_crop(
                CropRect {
                    x: 1800,
                    y: 80,
                    width: 800,
                    height: 600
                },
                &descriptor
            )
            .is_err()
        );
    }

    #[test]
    fn maps_display_crop_to_rotated_coded_edges() {
        let crop = CropRect {
            x: 100,
            y: 200,
            width: 800,
            height: 1400,
        };
        assert_eq!(
            coded_crop_edges(crop, &media(270)),
            CropEdges {
                left: 200,
                right: 320,
                top: 180,
                bottom: 100
            }
        );
    }

    #[test]
    fn parses_ffmpeg_microsecond_progress() {
        assert_eq!(parse_progress_time(b"out_time_us=3250000\n"), Some(3.25));
        assert_eq!(parse_progress_time(b"progress=continue\n"), None);
    }

    #[test]
    fn builds_compatible_physical_crop_arguments() {
        let args = build_export_args(
            Path::new("input.mp4"),
            Path::new("output.mp4"),
            CropRect {
                x: 100,
                y: 80,
                width: 800,
                height: 600,
            },
            &ExportSettings::default(),
            &media(0),
        );
        assert!(args.windows(2).any(|pair| pair == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|pair| pair == ["-crf", "17"]));
        assert!(args.contains(&"crop=w=800:h=600:x=100:y=80,setsar=1".to_owned()));
        assert!(
            !args.contains(&"copy".to_owned())
                || args.windows(2).any(|pair| pair == ["-c:a", "copy"])
        );
    }

    #[test]
    fn builds_lossless_ffv1_without_forcing_eight_bit_pixels() {
        let settings = ExportSettings {
            profile: ExportProfile::Lossless,
            pixel_format: PixelFormat::Source,
            ..ExportSettings::default()
        };
        let args = build_export_args(
            Path::new("input.mov"),
            Path::new("output.mkv"),
            CropRect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            &settings,
            &media(0),
        );
        assert!(args.windows(2).any(|pair| pair == ["-c:v", "ffv1"]));
        assert!(!args.contains(&"-pix_fmt".to_owned()));
    }

    #[test]
    fn metadata_profile_uses_copy_and_codec_crop_filter() {
        let settings = ExportSettings {
            profile: ExportProfile::Metadata,
            ..ExportSettings::default()
        };
        let args = build_export_args(
            Path::new("input.mp4"),
            Path::new("output.mp4"),
            CropRect {
                x: 100,
                y: 80,
                width: 800,
                height: 600,
            },
            &settings,
            &media(0),
        );
        assert!(args.windows(2).any(|pair| pair == ["-c", "copy"]));
        assert!(args.iter().any(|value| {
            value == "h264_metadata=crop_left=100:crop_right=1020:crop_top=80:crop_bottom=400"
        }));
        assert!(!args.contains(&"-vf".to_owned()));
    }

    #[test]
    fn applies_custom_codec_quality_audio_and_frame_rate() {
        let settings = ExportSettings {
            video_codec: VideoCodec::H265,
            crf: 23,
            preset: EncoderPreset::Slow,
            pixel_format: PixelFormat::Yuv420p10le,
            audio_mode: AudioMode::Aac,
            audio_bitrate_kbps: 256,
            frame_rate_mode: FrameRateMode::Constant,
            constant_frame_rate: 29.97,
            fast_start: false,
            preserve_metadata: false,
            ..ExportSettings::default()
        };
        let args = build_export_args(
            Path::new("input.mp4"),
            Path::new("output.mp4"),
            CropRect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            &settings,
            &media(0),
        );
        assert!(args.windows(2).any(|pair| pair == ["-c:v", "libx265"]));
        assert!(args.windows(2).any(|pair| pair == ["-crf", "23"]));
        assert!(args.windows(2).any(|pair| pair == ["-preset", "slow"]));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-pix_fmt", "yuv420p10le"])
        );
        assert!(args.windows(2).any(|pair| pair == ["-b:a", "256k"]));
        assert!(args.windows(2).any(|pair| pair == ["-r", "29.97"]));
        assert!(args.windows(2).any(|pair| pair == ["-map_metadata", "-1"]));
        assert!(!args.contains(&"+faststart".to_owned()));
    }

    #[test]
    fn rejects_invalid_detailed_settings() {
        let descriptor = media(0);
        let invalid_crf = ExportSettings {
            crf: 52,
            ..ExportSettings::default()
        };
        assert!(validate_export_settings(&invalid_crf, &descriptor).is_err());
        let invalid_mp4_audio = ExportSettings {
            audio_mode: AudioMode::Flac,
            ..ExportSettings::default()
        };
        assert!(validate_export_settings(&invalid_mp4_audio, &descriptor).is_err());
    }
}
