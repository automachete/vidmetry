use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, async_runtime::Receiver};
use tauri_plugin_shell::{
    ShellExt,
    process::{CommandChild, CommandEvent},
};
use uuid::Uuid;

use crate::{
    app_error::{AppError, ErrorCode},
    ffmpeg,
    media::MediaDescriptor,
};

const PROGRESS_EVENT: &str = "export-progress";
const COMPLETED_EVENT: &str = "export-complete";
const FAILED_EVENT: &str = "export-error";
const INPUT_SEEK_THRESHOLD_SECONDS: f64 = 30.0;
const INPUT_SEEK_PREROLL_SECONDS: f64 = 5.0;

#[derive(Default)]
pub struct ExportState {
    jobs: Mutex<HashMap<String, CommandChild>>,
    cancelled: Mutex<HashSet<String>>,
}

impl Drop for ExportState {
    fn drop(&mut self) {
        if let Ok(jobs) = self.jobs.get_mut() {
            for (_, child) in jobs.drain() {
                if let Err(error) = child.kill() {
                    log::warn!("unable to stop an export process during shutdown: {error}");
                }
            }
        } else {
            log::warn!("unable to access export jobs during shutdown");
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
pub enum VideoEncoder {
    Automatic,
    Nvidia,
    Intel,
    Amd,
    Software,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompatibleEncoder {
    Nvidia,
    Intel,
    Amd,
    Software,
}

impl CompatibleEncoder {
    fn is_hardware(self) -> bool {
        self != Self::Software
    }

    fn name(self) -> &'static str {
        match self {
            Self::Nvidia => "NVIDIA NVENC",
            Self::Intel => "Intel Quick Sync Video",
            Self::Amd => "AMD AMF",
            Self::Software => "software encoder",
        }
    }
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportSettings {
    pub profile: ExportProfile,
    pub video_codec: VideoCodec,
    pub encoder: VideoEncoder,
    pub crf: u8,
    pub preset: EncoderPreset,
    pub pixel_format: PixelFormat,
    pub audio_mode: AudioMode,
    pub audio_bitrate_kbps: u16,
    pub frame_rate_mode: FrameRateMode,
    pub constant_frame_rate: f64,
    pub preserve_metadata: bool,
    pub copy_subtitles: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrimRange {
    pub start_frame: u64,
    pub end_frame: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportRequest {
    pub source_path: String,
    pub output_path: String,
    pub crop: CropRect,
    pub trim: TrimRange,
    pub settings: ExportSettings,
    pub overwrite: bool,
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
    error: AppError,
    cancelled: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct HardwareEncoderAvailability {
    nvidia: bool,
    intel: bool,
    amd: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct VideoEncoderAvailability {
    h264: HardwareEncoderAvailability,
    h265: HardwareEncoderAvailability,
}

pub async fn available_video_encoders(app: AppHandle) -> VideoEncoderAvailability {
    let nvidia_app = app.clone();
    let nvidia = tauri::async_runtime::spawn(async move {
        (
            probe_video_encoder(&nvidia_app, VideoCodec::H264, CompatibleEncoder::Nvidia).await,
            probe_video_encoder(&nvidia_app, VideoCodec::H265, CompatibleEncoder::Nvidia).await,
        )
    });
    let intel_app = app.clone();
    let intel = tauri::async_runtime::spawn(async move {
        (
            probe_video_encoder(&intel_app, VideoCodec::H264, CompatibleEncoder::Intel).await,
            probe_video_encoder(&intel_app, VideoCodec::H265, CompatibleEncoder::Intel).await,
        )
    });
    let amd = tauri::async_runtime::spawn(async move {
        (
            probe_video_encoder(&app, VideoCodec::H264, CompatibleEncoder::Amd).await,
            probe_video_encoder(&app, VideoCodec::H265, CompatibleEncoder::Amd).await,
        )
    });

    let (nvidia_h264, nvidia_h265) = nvidia.await.unwrap_or((false, false));
    let (intel_h264, intel_h265) = intel.await.unwrap_or((false, false));
    let (amd_h264, amd_h265) = amd.await.unwrap_or((false, false));
    VideoEncoderAvailability {
        h264: HardwareEncoderAvailability {
            nvidia: nvidia_h264,
            intel: intel_h264,
            amd: amd_h264,
        },
        h265: HardwareEncoderAvailability {
            nvidia: nvidia_h265,
            intel: intel_h265,
            amd: amd_h265,
        },
    }
}

pub async fn start(
    app: AppHandle,
    state: State<'_, ExportState>,
    probe_cache: State<'_, ffmpeg::ProbeCache>,
    request: ExportRequest,
) -> Result<String, AppError> {
    let source = ffmpeg::canonical_source(&request.source_path).map_err(AppError::from)?;
    let output = validated_output(
        &source,
        &request.output_path,
        request.settings.profile,
        request.overwrite,
        request.in_place,
    )?;
    let media = ffmpeg::probe(&app, &probe_cache, &source)
        .await
        .map_err(AppError::from)?;
    validate_crop(request.crop, &media)?;
    let trim = request.trim;
    validate_trim(trim, &media, request.settings.profile)?;
    validate_export_settings(&request.settings, &media)?;

    let job_id = Uuid::new_v4().to_string();
    let temporary = temporary_output(&output, &job_id)?;
    let mut encoders = compatible_encoder_attempts(&request.settings);
    let mut encoder = encoders
        .pop_front()
        .expect("every export has at least one encoder attempt");
    let args = build_export_args(
        &source,
        &temporary,
        request.crop,
        trim,
        &request.settings,
        &media,
        encoder,
    );
    let (mut receiver, child) = spawn_export_process(&app, args)?;

    state
        .jobs
        .lock()
        .map_err(|_| AppError::new(ErrorCode::ExportStateUpdateFailed))?
        .insert(job_id.clone(), child);

    let task_app = app.clone();
    let task_job_id = job_id.clone();
    let duration = trim_duration_seconds(trim, &media);
    let overwrite = request.overwrite;
    let settings = request.settings;
    let crop = request.crop;
    tauri::async_runtime::spawn(async move {
        'attempts: loop {
            let mut diagnostics = Vec::new();
            let mut encoded_frame = false;
            while let Some(event) = receiver.recv().await {
                match event {
                    CommandEvent::Stdout(bytes) => {
                        if parse_progress_frame(&bytes).is_some_and(|frame| frame > 0) {
                            encoded_frame = true;
                        }
                        if let Some(seconds) = parse_progress_time(&bytes) {
                            let fraction = if duration > 0.0 {
                                (seconds / duration).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            if let Err(error) = task_app.emit(
                                PROGRESS_EVENT,
                                ExportProgress {
                                    job_id: task_job_id.clone(),
                                    fraction,
                                    out_time_seconds: seconds,
                                },
                            ) {
                                log::warn!("unable to emit export progress: {error}");
                            }
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
                        take_job(&task_app, &task_job_id);
                        let cancelled = take_cancelled(&task_app, &task_job_id);
                        if cancelled {
                            remove_partial_export(&temporary);
                            emit_failure(
                                &task_app,
                                &task_job_id,
                                AppError::new(ErrorCode::ExportCancelled),
                                true,
                            );
                            break 'attempts;
                        }

                        if should_retry_with_next_encoder(encoder, status.code, encoded_frame) {
                            remove_partial_export(&temporary);
                            let Some(next_encoder) = encoders.pop_front() else {
                                unreachable!("software fallback is always the final attempt");
                            };
                            log::warn!(
                                "{} could not start; retrying export with {}: {}",
                                encoder.name(),
                                next_encoder.name(),
                                diagnostic_detail(&diagnostics, status.code)
                            );
                            if take_cancelled(&task_app, &task_job_id) {
                                emit_failure(
                                    &task_app,
                                    &task_job_id,
                                    AppError::new(ErrorCode::ExportCancelled),
                                    true,
                                );
                                break 'attempts;
                            }
                            let args = build_export_args(
                                &source,
                                &temporary,
                                crop,
                                trim,
                                &settings,
                                &media,
                                next_encoder,
                            );
                            let (next_receiver, next_child) =
                                match spawn_export_process(&task_app, args) {
                                    Ok(process) => process,
                                    Err(error) => {
                                        emit_failure(&task_app, &task_job_id, error, false);
                                        break 'attempts;
                                    }
                                };
                            if let Err(error) = insert_job(&task_app, &task_job_id, next_child) {
                                emit_failure(&task_app, &task_job_id, error, false);
                                break 'attempts;
                            }
                            if take_cancelled(&task_app, &task_job_id) {
                                if let Some(child) = take_job(&task_app, &task_job_id)
                                    && let Err(error) = child.kill()
                                {
                                    log::warn!(
                                        "unable to stop a retried export after cancellation: {error}"
                                    );
                                }
                                remove_partial_export(&temporary);
                                emit_failure(
                                    &task_app,
                                    &task_job_id,
                                    AppError::new(ErrorCode::ExportCancelled),
                                    true,
                                );
                                break 'attempts;
                            }
                            receiver = next_receiver;
                            encoder = next_encoder;
                            continue 'attempts;
                        }

                        if status.code == Some(0) {
                            match commit_output(&temporary, &output, overwrite) {
                                Ok(()) => {
                                    task_app.state::<ffmpeg::ProbeCache>().invalidate(&output);
                                    if let Err(error) = task_app.emit(
                                        COMPLETED_EVENT,
                                        ExportCompleted {
                                            job_id: task_job_id.clone(),
                                            output_path: ffmpeg::display_path(&output),
                                        },
                                    ) {
                                        log::warn!("unable to emit export completion: {error}");
                                    }
                                }
                                Err(error) => {
                                    remove_partial_export(&temporary);
                                    emit_failure(&task_app, &task_job_id, error, false);
                                }
                            }
                        } else {
                            remove_partial_export(&temporary);
                            emit_failure(
                                &task_app,
                                &task_job_id,
                                AppError::with_detail(
                                    ErrorCode::ExportProcessFailed,
                                    diagnostic_detail(&diagnostics, status.code),
                                ),
                                false,
                            );
                        }
                        break 'attempts;
                    }
                    _ => {}
                }
            }
            if let Some(child) = take_job(&task_app, &task_job_id)
                && let Err(error) = child.kill()
            {
                log::warn!("unable to stop export after its event channel closed: {error}");
            }
            let cancelled = take_cancelled(&task_app, &task_job_id);
            remove_partial_export(&temporary);
            emit_failure(
                &task_app,
                &task_job_id,
                if cancelled {
                    AppError::new(ErrorCode::ExportCancelled)
                } else {
                    AppError::with_detail(
                        ErrorCode::ExportProcessFailed,
                        "process event channel closed",
                    )
                },
                cancelled,
            );
            break 'attempts;
        }
    });

    Ok(job_id)
}

pub fn cancel(state: State<'_, ExportState>, job_id: String) -> Result<(), AppError> {
    state
        .cancelled
        .lock()
        .map_err(|_| AppError::new(ErrorCode::CancellationStateUpdateFailed))?
        .insert(job_id.clone());
    let child = state
        .jobs
        .lock()
        .map_err(|_| AppError::new(ErrorCode::ExportStateReadFailed))?
        .remove(&job_id);
    let Some(child) = child else {
        return Ok(());
    };
    match child.kill() {
        Ok(()) => Ok(()),
        Err(error) => {
            if let Ok(mut cancelled) = state.cancelled.lock() {
                cancelled.remove(&job_id);
            }
            Err(AppError::with_detail(
                ErrorCode::ExportProcessStopFailed,
                error.to_string(),
            ))
        }
    }
}

fn spawn_export_process(
    app: &AppHandle,
    args: Vec<String>,
) -> Result<(Receiver<CommandEvent>, CommandChild), AppError> {
    app.shell()
        .sidecar("ffmpeg")
        .map_err(|error| {
            AppError::with_detail(ErrorCode::ExportProcessPrepareFailed, error.to_string())
        })?
        .args(args)
        .spawn()
        .map_err(|error| {
            AppError::with_detail(ErrorCode::ExportProcessStartFailed, error.to_string())
        })
}

async fn probe_video_encoder(
    app: &AppHandle,
    codec: VideoCodec,
    encoder: CompatibleEncoder,
) -> bool {
    let command = match app.shell().sidecar("ffmpeg") {
        Ok(command) => command,
        Err(error) => {
            log::debug!(
                "{} {:?} probe could not start: {error}",
                encoder.name(),
                codec
            );
            return false;
        }
    };
    match command
        .args(encoder_probe_args(codec, encoder))
        .output()
        .await
    {
        Ok(output) if output.status.success() => true,
        Ok(output) => {
            log::debug!(
                "{} {:?} probe failed: {}",
                encoder.name(),
                codec,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            false
        }
        Err(error) => {
            log::debug!(
                "{} {:?} probe could not start: {error}",
                encoder.name(),
                codec
            );
            false
        }
    }
}

fn encoder_probe_args(codec: VideoCodec, encoder: CompatibleEncoder) -> Vec<String> {
    vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        "color=c=black:s=256x256:r=1".into(),
        "-frames:v".into(),
        "1".into(),
        "-an".into(),
        "-c:v".into(),
        video_encoder(codec, encoder).into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-f".into(),
        "null".into(),
        "-".into(),
    ]
}

fn insert_job(app: &AppHandle, job_id: &str, child: CommandChild) -> Result<(), AppError> {
    match app.state::<ExportState>().jobs.lock() {
        Ok(mut jobs) => {
            jobs.insert(job_id.to_owned(), child);
            Ok(())
        }
        Err(_) => {
            if let Err(error) = child.kill() {
                log::warn!("unable to stop export after its state update failed: {error}");
            }
            Err(AppError::new(ErrorCode::ExportStateUpdateFailed))
        }
    }
}

fn diagnostic_detail(diagnostics: &[String], exit_code: Option<i32>) -> String {
    if diagnostics.is_empty() {
        exit_code
            .map(|code| format!("exit code {code}"))
            .unwrap_or_else(|| "no exit code".to_owned())
    } else {
        diagnostics.join(" ")
    }
}

fn should_retry_with_next_encoder(
    encoder: CompatibleEncoder,
    exit_code: Option<i32>,
    encoded_frame: bool,
) -> bool {
    encoder.is_hardware() && exit_code != Some(0) && !encoded_frame
}

fn build_export_args(
    source: &Path,
    output: &Path,
    crop: CropRect,
    trim: TrimRange,
    settings: &ExportSettings,
    media: &MediaDescriptor,
    encoder: CompatibleEncoder,
) -> Vec<String> {
    let time_trimmed = !is_full_trim(trim, media);
    let (trim_start, trim_end) = trim_times(trim, media);
    let input_seek = time_trimmed.then(|| input_seek(trim, media)).flatten();
    let filter_trim = time_trimmed.then(|| {
        input_seek
            .map(|seek| TrimRange {
                start_frame: trim.start_frame - seek.start_frame,
                end_frame: trim.end_frame - seek.start_frame,
            })
            .unwrap_or(trim)
    });
    let seek_seconds = input_seek.map(|seek| seek.seconds).unwrap_or(0.0);
    let filter_start = (trim_start - seek_seconds).max(0.0);
    let filter_end = (trim_end - seek_seconds).max(filter_start);
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
    if let Some(seek) = input_seek {
        args.extend(["-ss".into(), format_timestamp(seek.seconds)]);
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
                crop_filter(crop, filter_trim, media),
            ]);
            add_video_encoder(&mut args, settings, encoder);
            add_pixel_format(&mut args, settings.pixel_format);
            add_frame_rate(&mut args, settings);
            add_audio_trim_filter(
                &mut args,
                settings,
                media,
                time_trimmed,
                filter_start,
                filter_end,
            );
            add_audio(&mut args, settings, media, true, time_trimmed);
            add_metadata_mapping(&mut args, settings.preserve_metadata);
            args.extend([
                "-metadata:s:v:0".into(),
                "rotate=0".into(),
                "-movflags".into(),
                "+faststart".into(),
            ]);
        }
        ExportProfile::Lossless => {
            args.extend(["-map".into(), "0:v:0".into(), "-map".into(), "0:a?".into()]);
            if settings.copy_subtitles && !time_trimmed {
                args.extend(["-map".into(), "0:s?".into()]);
            }
            args.extend([
                "-vf".into(),
                crop_filter(crop, filter_trim, media),
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
            let parallelism = ffv1_parallelism(crop);
            args.extend([
                "-threads".into(),
                parallelism.threads.to_string(),
                "-slices".into(),
                parallelism.slices.to_string(),
            ]);
            add_pixel_format(&mut args, settings.pixel_format);
            add_frame_rate(&mut args, settings);
            add_audio_trim_filter(
                &mut args,
                settings,
                media,
                time_trimmed,
                filter_start,
                filter_end,
            );
            add_audio(&mut args, settings, media, false, time_trimmed);
            if settings.copy_subtitles && !time_trimmed {
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

    if time_trimmed {
        args.extend(["-t".into(), format_float(trim_end - trim_start)]);
    }

    args.push(output.to_string_lossy().into_owned());
    args
}

fn compatible_encoder_attempts(settings: &ExportSettings) -> VecDeque<CompatibleEncoder> {
    if settings.profile != ExportProfile::Compatible || settings.crf == 0 {
        return VecDeque::from([CompatibleEncoder::Software]);
    }
    match settings.encoder {
        VideoEncoder::Automatic => VecDeque::from([
            CompatibleEncoder::Nvidia,
            CompatibleEncoder::Intel,
            CompatibleEncoder::Amd,
            CompatibleEncoder::Software,
        ]),
        VideoEncoder::Nvidia => {
            VecDeque::from([CompatibleEncoder::Nvidia, CompatibleEncoder::Software])
        }
        VideoEncoder::Intel => {
            VecDeque::from([CompatibleEncoder::Intel, CompatibleEncoder::Software])
        }
        VideoEncoder::Amd => VecDeque::from([CompatibleEncoder::Amd, CompatibleEncoder::Software]),
        VideoEncoder::Software => VecDeque::from([CompatibleEncoder::Software]),
    }
}

fn video_encoder(codec: VideoCodec, encoder: CompatibleEncoder) -> &'static str {
    match (codec, encoder) {
        (VideoCodec::H264, CompatibleEncoder::Nvidia) => "h264_nvenc",
        (VideoCodec::H265, CompatibleEncoder::Nvidia) => "hevc_nvenc",
        (VideoCodec::H264, CompatibleEncoder::Intel) => "h264_qsv",
        (VideoCodec::H265, CompatibleEncoder::Intel) => "hevc_qsv",
        (VideoCodec::H264, CompatibleEncoder::Amd) => "h264_amf",
        (VideoCodec::H265, CompatibleEncoder::Amd) => "hevc_amf",
        (VideoCodec::H264, CompatibleEncoder::Software) => "libx264",
        (VideoCodec::H265, CompatibleEncoder::Software) => "libx265",
    }
}

fn add_video_encoder(
    args: &mut Vec<String>,
    settings: &ExportSettings,
    encoder: CompatibleEncoder,
) {
    args.extend([
        "-c:v".into(),
        video_encoder(settings.video_codec, encoder).into(),
    ]);
    match encoder {
        CompatibleEncoder::Nvidia => args.extend([
            "-preset".into(),
            nvenc_preset(settings.preset).into(),
            "-tune".into(),
            "hq".into(),
            "-rc".into(),
            "vbr".into(),
            "-cq".into(),
            settings.crf.to_string(),
            "-b:v".into(),
            "0".into(),
        ]),
        CompatibleEncoder::Intel => args.extend([
            "-preset".into(),
            qsv_preset(settings.preset).into(),
            "-global_quality".into(),
            settings.crf.to_string(),
        ]),
        CompatibleEncoder::Amd => args.extend([
            "-usage".into(),
            "transcoding".into(),
            "-quality".into(),
            amf_quality(settings.preset).into(),
            "-rc".into(),
            "qvbr".into(),
            "-qvbr_quality_level".into(),
            settings.crf.to_string(),
        ]),
        CompatibleEncoder::Software => args.extend([
            "-preset".into(),
            encoder_preset(settings.preset).into(),
            "-crf".into(),
            settings.crf.to_string(),
        ]),
    }
}

fn nvenc_preset(preset: EncoderPreset) -> &'static str {
    match preset {
        EncoderPreset::Ultrafast | EncoderPreset::Superfast => "p1",
        EncoderPreset::Veryfast => "p2",
        EncoderPreset::Faster => "p3",
        EncoderPreset::Fast | EncoderPreset::Medium => "p4",
        EncoderPreset::Slow => "p5",
        EncoderPreset::Slower => "p6",
        EncoderPreset::Veryslow => "p7",
    }
}

fn qsv_preset(preset: EncoderPreset) -> &'static str {
    match preset {
        EncoderPreset::Ultrafast | EncoderPreset::Superfast | EncoderPreset::Veryfast => "veryfast",
        EncoderPreset::Faster => "faster",
        EncoderPreset::Fast => "fast",
        EncoderPreset::Medium => "medium",
        EncoderPreset::Slow => "slow",
        EncoderPreset::Slower => "slower",
        EncoderPreset::Veryslow => "veryslow",
    }
}

fn amf_quality(preset: EncoderPreset) -> &'static str {
    match preset {
        EncoderPreset::Ultrafast | EncoderPreset::Superfast | EncoderPreset::Veryfast => "speed",
        EncoderPreset::Faster | EncoderPreset::Fast | EncoderPreset::Medium => "balanced",
        EncoderPreset::Slow | EncoderPreset::Slower => "quality",
        EncoderPreset::Veryslow => "high_quality",
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
    time_trimmed: bool,
) {
    let mut mode = match settings.audio_mode {
        AudioMode::Auto if compatible && media.audio_codec.as_deref() == Some("aac") => {
            AudioMode::Copy
        }
        AudioMode::Auto if compatible => AudioMode::Aac,
        AudioMode::Auto => AudioMode::Copy,
        selected => selected,
    };
    if time_trimmed && mode == AudioMode::Copy {
        mode = if compatible {
            AudioMode::Aac
        } else {
            AudioMode::Flac
        };
    }
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

fn add_audio_trim_filter(
    args: &mut Vec<String>,
    settings: &ExportSettings,
    media: &MediaDescriptor,
    time_trimmed: bool,
    start: f64,
    end: f64,
) {
    if time_trimmed && media.has_audio && settings.audio_mode != AudioMode::None {
        args.extend([
            "-af".into(),
            format!(
                "atrim=start={}:end={},asetpts=PTS-STARTPTS",
                format_float(start),
                format_float(end)
            ),
        ]);
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

fn format_timestamp(value: f64) -> String {
    let text = format!("{value:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn crop_filter(crop: CropRect, trim: Option<TrimRange>, media: &MediaDescriptor) -> String {
    let mut filter = format!(
        "crop=w={}:h={}:x={}:y={},setsar=1",
        crop.width, crop.height, crop.x, crop.y
    );
    if let Some(trim) = trim {
        filter.push_str(&format!(
            ",trim=start_frame={}:end_frame={},setpts=PTS-STARTPTS",
            trim.start_frame, trim.end_frame
        ));
    }
    let mut color_values = Vec::new();
    for (name, value) in [
        ("color_primaries", media.color.primaries.as_deref()),
        ("color_trc", media.color.transfer.as_deref()),
        ("colorspace", media.color.matrix.as_deref()),
        ("range", media.color.range.as_deref()),
    ] {
        let Some(value) = value.and_then(color_filter_value) else {
            continue;
        };
        color_values.push(format!("{name}={value}"));
    }
    if !color_values.is_empty() {
        filter.push_str(",setparams=");
        filter.push_str(&color_values.join(":"));
    }
    filter
}

fn color_filter_value(value: &str) -> Option<&str> {
    let value = match value {
        "tv" => "limited",
        "pc" => "full",
        value => value,
    };
    (!value.is_empty()
        && value != "unknown"
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')))
    .then_some(value)
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

fn validate_crop(crop: CropRect, media: &MediaDescriptor) -> Result<(), AppError> {
    if crop.width < 16 || crop.height < 16 {
        return Err(AppError::new(ErrorCode::CropTooSmall));
    }
    if [crop.x, crop.y, crop.width, crop.height]
        .into_iter()
        .any(|value| value % 2 != 0)
    {
        return Err(AppError::new(ErrorCode::CropEvenValuesRequired));
    }
    let right = crop
        .x
        .checked_add(crop.width)
        .ok_or_else(|| AppError::new(ErrorCode::CropInvalid))?;
    let bottom = crop
        .y
        .checked_add(crop.height)
        .ok_or_else(|| AppError::new(ErrorCode::CropInvalid))?;
    if right > media.display_width || bottom > media.display_height {
        return Err(AppError::new(ErrorCode::CropOutsideVideo));
    }
    Ok(())
}

fn total_frames(media: &MediaDescriptor) -> u64 {
    media.frame_count
}

fn full_trim(media: &MediaDescriptor) -> TrimRange {
    TrimRange {
        start_frame: 0,
        end_frame: total_frames(media),
    }
}

fn is_full_trim(trim: TrimRange, media: &MediaDescriptor) -> bool {
    trim == full_trim(media)
}

fn trim_times(trim: TrimRange, media: &MediaDescriptor) -> (f64, f64) {
    let total = total_frames(media) as f64;
    let duration = media.duration_seconds.max(0.0);
    (
        trim.start_frame as f64 / total * duration,
        trim.end_frame as f64 / total * duration,
    )
}

fn trim_duration_seconds(trim: TrimRange, media: &MediaDescriptor) -> f64 {
    let (start, end) = trim_times(trim, media);
    (end - start).max(0.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct InputSeek {
    start_frame: u64,
    seconds: f64,
}

fn input_seek(trim: TrimRange, media: &MediaDescriptor) -> Option<InputSeek> {
    if !media.frame_seek_supported || is_full_trim(trim, media) {
        return None;
    }
    let total = total_frames(media);
    let duration = media.duration_seconds;
    if total == 0 || duration <= 0.0 {
        return None;
    }
    let trim_start_seconds = trim.start_frame as f64 / total as f64 * duration;
    if trim_start_seconds < INPUT_SEEK_THRESHOLD_SECONDS {
        return None;
    }
    let frames_per_second = total as f64 / duration;
    let preroll_frames = (frames_per_second * INPUT_SEEK_PREROLL_SECONDS).ceil() as u64;
    let start_frame = trim.start_frame.saturating_sub(preroll_frames);
    if start_frame == 0 {
        return None;
    }
    Some(InputSeek {
        start_frame,
        seconds: start_frame as f64 / total as f64 * duration,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ffv1Parallelism {
    threads: usize,
    slices: usize,
}

fn ffv1_parallelism(crop: CropRect) -> Ffv1Parallelism {
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    ffv1_parallelism_for_workers(crop, workers)
}

fn ffv1_parallelism_for_workers(crop: CropRect, workers: usize) -> Ffv1Parallelism {
    let pixels = u64::from(crop.width) * u64::from(crop.height);
    let slice_limit = if pixels >= 3840_u64 * 2160 {
        192
    } else if pixels >= 1920_u64 * 1080 {
        128
    } else if pixels >= 1280_u64 * 720 {
        96
    } else if pixels >= 640_u64 * 360 {
        64
    } else {
        32
    };
    let threads = workers.clamp(1, 64);
    let slices_per_thread = if pixels >= 3840_u64 * 2160 { 6 } else { 4 };
    Ffv1Parallelism {
        threads,
        slices: threads
            .saturating_mul(slices_per_thread)
            .clamp(4, slice_limit),
    }
}

fn validate_trim(
    trim: TrimRange,
    media: &MediaDescriptor,
    profile: ExportProfile,
) -> Result<(), AppError> {
    let total = total_frames(media);
    if trim.start_frame >= trim.end_frame || trim.end_frame > total {
        return Err(AppError::new(ErrorCode::TrimOutsideVideo));
    }
    if profile == ExportProfile::Metadata && !is_full_trim(trim, media) {
        return Err(AppError::new(ErrorCode::MetadataTrimUnsupported));
    }
    Ok(())
}

fn validate_export_settings(
    settings: &ExportSettings,
    media: &MediaDescriptor,
) -> Result<(), AppError> {
    if settings.profile == ExportProfile::Metadata && !media.metadata_crop_supported {
        return Err(AppError::new(ErrorCode::MetadataCodecUnsupported));
    }
    if settings.crf > 51 {
        return Err(AppError::new(ErrorCode::CrfOutOfRange));
    }
    if !(32..=1024).contains(&settings.audio_bitrate_kbps) {
        return Err(AppError::new(ErrorCode::AudioBitrateOutOfRange));
    }
    if settings.frame_rate_mode == FrameRateMode::Constant
        && (!settings.constant_frame_rate.is_finite()
            || !(1.0..=240.0).contains(&settings.constant_frame_rate))
    {
        return Err(AppError::new(ErrorCode::FrameRateOutOfRange));
    }
    if settings.profile == ExportProfile::Compatible
        && matches!(settings.audio_mode, AudioMode::Flac | AudioMode::Pcm)
    {
        return Err(AppError::new(ErrorCode::CompatibleAudioUnsupported));
    }
    Ok(())
}

fn validated_output(
    source: &Path,
    requested: &str,
    profile: ExportProfile,
    overwrite: bool,
    in_place: bool,
) -> Result<PathBuf, AppError> {
    let output = PathBuf::from(requested);
    if !output.is_absolute() {
        return Err(AppError::new(ErrorCode::DestinationMustBeAbsolute));
    }
    let parent = output
        .parent()
        .ok_or_else(|| AppError::new(ErrorCode::DestinationFolderUnavailable))?;
    let canonical_parent = fs::canonicalize(parent).map_err(|error| {
        AppError::with_detail(ErrorCode::DestinationFolderUnavailable, error.to_string())
    })?;
    if !canonical_parent.is_dir() {
        return Err(AppError::new(ErrorCode::DestinationFolderMissing));
    }
    let file_name = output
        .file_name()
        .ok_or_else(|| AppError::new(ErrorCode::DestinationFileNameMissing))?;
    let output = canonical_parent.join(file_name);
    let mut matches_source = false;
    if output.exists() {
        let existing = fs::canonicalize(&output).map_err(|error| {
            AppError::with_detail(
                ErrorCode::DestinationFileInspectionFailed,
                error.to_string(),
            )
        })?;
        matches_source = existing == source;
        if matches_source && !in_place {
            return Err(AppError::new(ErrorCode::SourceReplacementRequiresSave));
        }
        if !matches_source && !overwrite {
            return Err(AppError::new(ErrorCode::DestinationAlreadyExists));
        }
    }
    if in_place && !matches_source {
        return Err(AppError::new(ErrorCode::SaveDestinationMismatch));
    }
    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| AppError::new(ErrorCode::DestinationExtensionMissing))?;
    let valid = match profile {
        ExportProfile::Compatible => extension == "mp4",
        ExportProfile::Lossless => extension == "mkv",
        ExportProfile::Metadata => ["mp4", "m4v", "mov", "mkv"].contains(&extension.as_str()),
    };
    if !valid {
        return Err(AppError::new(match profile {
            ExportProfile::Compatible => ErrorCode::CompatibleExtensionRequired,
            ExportProfile::Lossless => ErrorCode::LosslessExtensionRequired,
            ExportProfile::Metadata => ErrorCode::MetadataExtensionUnsupported,
        }));
    }
    Ok(output)
}

fn temporary_output(output: &Path, job_id: &str) -> Result<PathBuf, AppError> {
    let parent = output
        .parent()
        .ok_or_else(|| AppError::new(ErrorCode::DestinationFolderUnavailable))?;
    let stem = output
        .file_stem()
        .map(|value| value.to_string_lossy())
        .ok_or_else(|| AppError::new(ErrorCode::DestinationFileNameMissing))?;
    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::new(ErrorCode::DestinationExtensionMissing))?;
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

fn parse_progress_frame(bytes: &[u8]) -> Option<u64> {
    String::from_utf8_lossy(bytes)
        .strip_prefix("frame=")?
        .trim()
        .parse()
        .ok()
}

fn take_job(app: &AppHandle, job_id: &str) -> Option<CommandChild> {
    match app.state::<ExportState>().jobs.lock() {
        Ok(mut jobs) => jobs.remove(job_id),
        Err(_) => {
            log::warn!("unable to access export jobs while finishing {job_id}");
            None
        }
    }
}

fn take_cancelled(app: &AppHandle, job_id: &str) -> bool {
    app.state::<ExportState>()
        .cancelled
        .lock()
        .map(|mut jobs| jobs.remove(job_id))
        .unwrap_or_else(|_| {
            log::warn!("unable to access export cancellation state for {job_id}");
            false
        })
}

fn emit_failure(app: &AppHandle, job_id: &str, error: AppError, cancelled: bool) {
    if let Err(error) = app.emit(
        FAILED_EVENT,
        ExportFailed {
            job_id: job_id.to_owned(),
            error,
            cancelled,
        },
    ) {
        log::warn!("unable to emit export failure: {error}");
    }
}

fn remove_partial_export(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => log::warn!(
            "unable to remove partial export {}: {error}",
            path.display()
        ),
    }
}

#[cfg(windows)]
fn commit_output(temporary: &Path, output: &Path, overwrite: bool) -> Result<(), AppError> {
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
        Err(AppError::with_detail(
            ErrorCode::CommitOutputFailed,
            std::io::Error::last_os_error().to_string(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn commit_output(temporary: &Path, output: &Path, overwrite: bool) -> Result<(), AppError> {
    if output.exists() && !overwrite {
        return Err(AppError::new(ErrorCode::DestinationAlreadyExists));
    }
    fs::rename(temporary, output)
        .map_err(|error| AppError::with_detail(ErrorCode::CommitOutputFailed, error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::ColorDescriptor;

    fn default_settings() -> ExportSettings {
        ExportSettings {
            profile: ExportProfile::Compatible,
            video_codec: VideoCodec::H264,
            encoder: VideoEncoder::Automatic,
            crf: 17,
            preset: EncoderPreset::Medium,
            pixel_format: PixelFormat::Yuv420p,
            audio_mode: AudioMode::Auto,
            audio_bitrate_kbps: 192,
            frame_rate_mode: FrameRateMode::Passthrough,
            constant_frame_rate: 30.0,
            preserve_metadata: true,
            copy_subtitles: true,
        }
    }

    fn request_json() -> serde_json::Value {
        serde_json::json!({
            "sourcePath": "C:\\input.mp4",
            "outputPath": "C:\\output.mp4",
            "crop": { "x": 0, "y": 0, "width": 1920, "height": 1080 },
            "trim": { "startFrame": 0, "endFrame": 360 },
            "settings": {
                "profile": "compatible",
                "videoCodec": "h264",
                "encoder": "automatic",
                "crf": 17,
                "preset": "medium",
                "pixelFormat": "yuv420p",
                "audioMode": "auto",
                "audioBitrateKbps": 192,
                "frameRateMode": "passthrough",
                "constantFrameRate": 30,
                "preserveMetadata": true,
                "copySubtitles": true
            },
            "overwrite": true,
            "inPlace": false
        })
    }

    #[test]
    fn rejects_incomplete_or_obsolete_export_request_shapes() {
        let mut missing_trim = request_json();
        missing_trim.as_object_mut().unwrap().remove("trim");
        assert!(serde_json::from_value::<ExportRequest>(missing_trim).is_err());

        let mut obsolete_extra = request_json();
        obsolete_extra["legacyMode"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ExportRequest>(obsolete_extra).is_err());

        let mut obsolete_fast_start = request_json();
        obsolete_fast_start["settings"]["fastStart"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ExportRequest>(obsolete_fast_start).is_err());

        let mut missing_encoder = request_json();
        missing_encoder["settings"]
            .as_object_mut()
            .unwrap()
            .remove("encoder");
        assert!(serde_json::from_value::<ExportRequest>(missing_encoder).is_err());

        let mut invalid_encoder = request_json();
        invalid_encoder["settings"]["encoder"] = serde_json::json!("unknown");
        assert!(serde_json::from_value::<ExportRequest>(invalid_encoder).is_err());

        assert!(serde_json::from_value::<ExportRequest>(request_json()).is_ok());
    }

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
            frame_count: 360,
            coded_width: 1920,
            coded_height: 1080,
            display_width,
            display_height,
            rotation_degrees,
            frame_rate: "30/1".into(),
            frame_seek_supported: true,
            video_codec: "h264".into(),
            pixel_format: "yuv420p".into(),
            bit_depth: Some(8),
            has_audio: true,
            audio_codec: Some("aac".into()),
            color: ColorDescriptor {
                primaries: Some("bt709".into()),
                transfer: Some("bt709".into()),
                matrix: Some("bt709".into()),
                range: Some("tv".into()),
            },
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
        assert_eq!(parse_progress_frame(b"frame=42\n"), Some(42));
        assert_eq!(parse_progress_frame(b"fps=60.0\n"), None);
    }

    #[test]
    fn tries_hardware_encoders_before_the_software_fallback() {
        assert_eq!(
            compatible_encoder_attempts(&default_settings())
                .into_iter()
                .collect::<Vec<_>>(),
            vec![
                CompatibleEncoder::Nvidia,
                CompatibleEncoder::Intel,
                CompatibleEncoder::Amd,
                CompatibleEncoder::Software,
            ]
        );

        let lossless_quality = ExportSettings {
            crf: 0,
            ..default_settings()
        };
        assert_eq!(
            compatible_encoder_attempts(&lossless_quality)
                .into_iter()
                .collect::<Vec<_>>(),
            vec![CompatibleEncoder::Software]
        );

        let selected_amd = ExportSettings {
            encoder: VideoEncoder::Amd,
            ..default_settings()
        };
        assert_eq!(
            compatible_encoder_attempts(&selected_amd)
                .into_iter()
                .collect::<Vec<_>>(),
            vec![CompatibleEncoder::Amd, CompatibleEncoder::Software]
        );

        let selected_software = ExportSettings {
            encoder: VideoEncoder::Software,
            ..default_settings()
        };
        assert_eq!(
            compatible_encoder_attempts(&selected_software)
                .into_iter()
                .collect::<Vec<_>>(),
            vec![CompatibleEncoder::Software]
        );

        assert!(should_retry_with_next_encoder(
            CompatibleEncoder::Nvidia,
            Some(1),
            false
        ));
        assert!(!should_retry_with_next_encoder(
            CompatibleEncoder::Nvidia,
            Some(1),
            true
        ));
        assert!(!should_retry_with_next_encoder(
            CompatibleEncoder::Software,
            Some(1),
            false
        ));
    }

    #[test]
    fn maps_the_existing_codec_quality_and_preset_to_hardware_encoders() {
        let crop = CropRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let descriptor = media(0);
        let h264 = default_settings();
        let nvenc = build_export_args(
            Path::new("input.mp4"),
            Path::new("output.mp4"),
            crop,
            full_trim(&descriptor),
            &h264,
            &descriptor,
            CompatibleEncoder::Nvidia,
        );
        assert!(nvenc.windows(2).any(|pair| pair == ["-c:v", "h264_nvenc"]));
        assert!(nvenc.windows(2).any(|pair| pair == ["-preset", "p4"]));
        assert!(nvenc.windows(2).any(|pair| pair == ["-cq", "17"]));
        assert!(!nvenc.contains(&"-crf".to_owned()));

        let h265 = ExportSettings {
            video_codec: VideoCodec::H265,
            crf: 23,
            preset: EncoderPreset::Slow,
            ..default_settings()
        };
        let qsv = build_export_args(
            Path::new("input.mp4"),
            Path::new("output.mp4"),
            crop,
            full_trim(&descriptor),
            &h265,
            &descriptor,
            CompatibleEncoder::Intel,
        );
        assert!(qsv.windows(2).any(|pair| pair == ["-c:v", "hevc_qsv"]));
        assert!(qsv.windows(2).any(|pair| pair == ["-preset", "slow"]));
        assert!(qsv.windows(2).any(|pair| pair == ["-global_quality", "23"]));

        let amf = build_export_args(
            Path::new("input.mp4"),
            Path::new("output.mp4"),
            crop,
            full_trim(&descriptor),
            &h265,
            &descriptor,
            CompatibleEncoder::Amd,
        );
        assert!(amf.windows(2).any(|pair| pair == ["-c:v", "hevc_amf"]));
        assert!(amf.windows(2).any(|pair| pair == ["-quality", "quality"]));
        assert!(
            amf.windows(2)
                .any(|pair| pair == ["-qvbr_quality_level", "23"])
        );
    }

    #[test]
    fn builds_a_single_frame_hardware_encoder_probe() {
        let arguments = encoder_probe_args(VideoCodec::H265, CompatibleEncoder::Nvidia);
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["-c:v", "hevc_nvenc"])
        );
        assert!(arguments.windows(2).any(|pair| pair == ["-frames:v", "1"]));
        assert!(arguments.windows(2).any(|pair| pair == ["-f", "null"]));
        assert_eq!(arguments.last().map(String::as_str), Some("-"));
    }

    #[test]
    fn accepts_only_ffmpeg_enum_tokens_in_color_filters() {
        assert_eq!(color_filter_value("tv"), Some("limited"));
        assert_eq!(color_filter_value("pc"), Some("full"));
        assert_eq!(color_filter_value("bt2020nc"), Some("bt2020nc"));
        assert_eq!(color_filter_value("unknown"), None);
        assert_eq!(color_filter_value("bt709,setpts=0"), None);
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
            full_trim(&media(0)),
            &default_settings(),
            &media(0),
            CompatibleEncoder::Software,
        );
        assert!(args.windows(2).any(|pair| pair == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|pair| pair == ["-crf", "17"]));
        assert!(args.contains(&"crop=w=800:h=600:x=100:y=80,setsar=1,setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=limited".to_owned()));
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
            ..default_settings()
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
            full_trim(&media(0)),
            &settings,
            &media(0),
            CompatibleEncoder::Software,
        );
        assert!(args.windows(2).any(|pair| pair == ["-c:v", "ffv1"]));
        assert!(args.windows(2).any(|pair| {
            pair[0] == "-threads" && pair[1].parse::<usize>().is_ok_and(|value| value > 0)
        }));
        assert!(args.windows(2).any(|pair| {
            pair[0] == "-slices" && pair[1].parse::<usize>().is_ok_and(|value| value > 1)
        }));
        assert!(!args.contains(&"-pix_fmt".to_owned()));
    }

    #[test]
    fn scales_ffv1_slice_parallelism_with_workers_and_output_size() {
        assert_eq!(
            ffv1_parallelism_for_workers(
                CropRect {
                    x: 0,
                    y: 0,
                    width: 3840,
                    height: 2160,
                },
                32,
            ),
            Ffv1Parallelism {
                threads: 32,
                slices: 192,
            }
        );
        assert_eq!(
            ffv1_parallelism_for_workers(
                CropRect {
                    x: 0,
                    y: 0,
                    width: 640,
                    height: 360,
                },
                32,
            ),
            Ffv1Parallelism {
                threads: 32,
                slices: 64,
            }
        );
    }

    #[test]
    fn metadata_profile_uses_copy_and_codec_crop_filter() {
        let settings = ExportSettings {
            profile: ExportProfile::Metadata,
            ..default_settings()
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
            full_trim(&media(0)),
            &settings,
            &media(0),
            CompatibleEncoder::Software,
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
            preserve_metadata: false,
            ..default_settings()
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
            full_trim(&media(0)),
            &settings,
            &media(0),
            CompatibleEncoder::Software,
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
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-movflags", "+faststart"])
        );
    }

    #[test]
    fn rejects_invalid_detailed_settings() {
        let descriptor = media(0);
        let invalid_crf = ExportSettings {
            crf: 52,
            ..default_settings()
        };
        assert!(validate_export_settings(&invalid_crf, &descriptor).is_err());
        let invalid_mp4_audio = ExportSettings {
            audio_mode: AudioMode::Flac,
            ..default_settings()
        };
        assert!(validate_export_settings(&invalid_mp4_audio, &descriptor).is_err());
    }

    #[test]
    fn builds_frame_exact_time_trim_and_reencodes_packet_audio() {
        let descriptor = media(0);
        let args = build_export_args(
            Path::new("input.mp4"),
            Path::new("output.mp4"),
            CropRect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            TrimRange {
                start_frame: 30,
                end_frame: 90,
            },
            &default_settings(),
            &descriptor,
            CompatibleEncoder::Software,
        );

        assert!(args.iter().any(|value| {
            value.contains("trim=start_frame=30:end_frame=90,setpts=PTS-STARTPTS")
        }));
        assert!(
            args.iter()
                .any(|value| { value == "atrim=start=1:end=3,asetpts=PTS-STARTPTS" })
        );
        assert!(args.windows(2).any(|pair| pair == ["-c:a", "aac"]));
        assert!(args.windows(2).any(|pair| pair == ["-t", "2"]));
    }

    #[test]
    fn seeks_near_late_cfr_trims_but_keeps_vfr_on_the_exact_full_decode_path() {
        let mut descriptor = media(0);
        descriptor.duration_seconds = 120.0;
        descriptor.frame_count = 3600;
        let trim = TrimRange {
            start_frame: 3000,
            end_frame: 3060,
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
            trim,
            &default_settings(),
            &descriptor,
            CompatibleEncoder::Software,
        );
        let seek_index = args.iter().position(|value| value == "-ss").unwrap();
        let input_index = args.iter().position(|value| value == "-i").unwrap();
        assert!(seek_index < input_index);
        assert_eq!(args[seek_index + 1], "95");
        assert!(args.iter().any(|value| {
            value.contains("trim=start_frame=150:end_frame=210,setpts=PTS-STARTPTS")
        }));
        assert!(
            args.iter()
                .any(|value| value == "atrim=start=5:end=7,asetpts=PTS-STARTPTS")
        );

        descriptor.frame_seek_supported = false;
        let vfr_args = build_export_args(
            Path::new("input.mp4"),
            Path::new("output.mp4"),
            CropRect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            trim,
            &default_settings(),
            &descriptor,
            CompatibleEncoder::Software,
        );
        assert!(!vfr_args.contains(&"-ss".to_owned()));
        assert!(vfr_args.iter().any(|value| {
            value.contains("trim=start_frame=3000:end_frame=3060,setpts=PTS-STARTPTS")
        }));
    }

    #[test]
    fn rejects_time_trim_for_metadata_only_stream_copy() {
        let descriptor = media(0);
        let trimmed = TrimRange {
            start_frame: 1,
            end_frame: 360,
        };
        assert!(validate_trim(trimmed, &descriptor, ExportProfile::Metadata).is_err());
        assert!(
            validate_trim(full_trim(&descriptor), &descriptor, ExportProfile::Metadata).is_ok()
        );
    }
}
