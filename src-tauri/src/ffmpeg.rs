use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    cache,
    media::{ColorDescriptor, MediaDescriptor, ProbeDocument, ProbeStream},
};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;

#[derive(Debug)]
pub enum MediaError {
    InvalidSource(String),
    ProcessStart { tool: &'static str, message: String },
    ProcessFailed { tool: &'static str, message: String },
    InvalidProbe(String),
    MissingVideo,
    MissingVideoWidth,
    MissingVideoHeight,
    MissingVideoDuration,
    MissingVideoFrameCount,
    MissingVideoFrameRate,
    Cache(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFingerprint {
    length: u64,
    modified: SystemTime,
}

#[derive(Debug, Clone)]
struct CachedProbe {
    source: PathBuf,
    fingerprint: SourceFingerprint,
    descriptor: MediaDescriptor,
}

#[derive(Default)]
pub struct ProbeCache {
    latest: Mutex<Option<CachedProbe>>,
}

impl ProbeCache {
    fn get(&self, source: &Path, fingerprint: &SourceFingerprint) -> Option<MediaDescriptor> {
        match self.latest.lock() {
            Ok(cache) => cache
                .as_ref()
                .filter(|cached| cached.source == source && cached.fingerprint == *fingerprint)
                .map(|cached| cached.descriptor.clone()),
            Err(_) => {
                log::warn!("unable to read the media probe cache");
                None
            }
        }
    }

    fn store(&self, source: &Path, fingerprint: SourceFingerprint, descriptor: &MediaDescriptor) {
        match self.latest.lock() {
            Ok(mut cache) => {
                *cache = Some(CachedProbe {
                    source: source.to_owned(),
                    fingerprint,
                    descriptor: descriptor.clone(),
                });
            }
            Err(_) => log::warn!("unable to update the media probe cache"),
        }
    }

    pub fn invalidate(&self, source: &Path) {
        match self.latest.lock() {
            Ok(mut cache) => {
                if cache.as_ref().is_some_and(|cached| cached.source == source) {
                    *cache = None;
                }
            }
            Err(_) => log::warn!("unable to invalidate the media probe cache"),
        }
    }
}

pub fn canonical_source(path: &str) -> Result<PathBuf, MediaError> {
    let source = PathBuf::from(path);
    let canonical = fs::canonicalize(&source)
        .map_err(|error| MediaError::InvalidSource(format!("{path}: {error}")))?;
    if !canonical.is_file() {
        return Err(MediaError::InvalidSource(path.to_owned()));
    }
    Ok(canonical)
}

pub fn display_path(path: &Path) -> String {
    let raw = path.to_string_lossy();

    #[cfg(windows)]
    {
        if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = raw.strip_prefix(r"\\?\") {
            return rest.to_owned();
        }
    }

    raw.into_owned()
}

pub async fn probe(
    app: &AppHandle,
    cache: &ProbeCache,
    source: &Path,
) -> Result<MediaDescriptor, MediaError> {
    let fingerprint = source_fingerprint(source)?;
    if let Some(fingerprint) = fingerprint.as_ref()
        && let Some(descriptor) = cache.get(source, fingerprint)
    {
        return Ok(descriptor);
    }
    let descriptor = probe_uncached(app, source).await?;
    if let Some(fingerprint) = fingerprint {
        cache.store(source, fingerprint, &descriptor);
    }
    Ok(descriptor)
}

async fn probe_uncached(app: &AppHandle, source: &Path) -> Result<MediaDescriptor, MediaError> {
    let initial = run_probe(app, source, false).await?;
    match descriptor_from_probe(source, initial) {
        Err(MediaError::MissingVideoFrameCount) => {
            descriptor_from_probe(source, run_probe(app, source, true).await?)
        }
        result => result,
    }
}

fn source_fingerprint(source: &Path) -> Result<Option<SourceFingerprint>, MediaError> {
    let metadata = source
        .metadata()
        .map_err(|error| MediaError::InvalidSource(format!("{}: {error}", source.display())))?;
    let Ok(modified) = metadata.modified() else {
        log::warn!(
            "media probe cache disabled because the modification time is unavailable: {}",
            source.display()
        );
        return Ok(None);
    };
    Ok(Some(SourceFingerprint {
        length: metadata.len(),
        modified,
    }))
}

async fn run_probe(
    app: &AppHandle,
    source: &Path,
    count_frames: bool,
) -> Result<ProbeDocument, MediaError> {
    let source_text = source.to_string_lossy().into_owned();
    let mut arguments = vec!["-v".to_owned(), "error".to_owned()];
    if count_frames {
        arguments.push("-count_frames".to_owned());
    }
    arguments.extend([
        "-show_streams".to_owned(),
        "-show_format".to_owned(),
        "-of".to_owned(),
        "json".to_owned(),
        source_text,
    ]);
    let output = app
        .shell()
        .sidecar("ffprobe")
        .map_err(|error| MediaError::ProcessStart {
            tool: "ffprobe",
            message: error.to_string(),
        })?
        .args(arguments)
        .output()
        .await
        .map_err(|error| MediaError::ProcessStart {
            tool: "ffprobe",
            message: error.to_string(),
        })?;

    if !output.status.success() {
        return Err(MediaError::ProcessFailed {
            tool: "ffprobe",
            message: compact_stderr(&output.stderr),
        });
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|error| MediaError::InvalidProbe(error.to_string()))
}

pub async fn create_preview(app: &AppHandle, source: &Path) -> Result<PathBuf, MediaError> {
    let cache_root = app
        .path()
        .app_cache_dir()
        .map_err(|error| MediaError::Cache(error.to_string()))?
        .join("previews");
    fs::create_dir_all(&cache_root).map_err(|error| MediaError::Cache(error.to_string()))?;

    let output_path = cache_root.join(format!("{:016x}.mp4", source_cache_key(source)?));
    if cache::reusable_entry(&output_path).map_err(|error| MediaError::Cache(error.to_string()))? {
        prune_cache(&cache_root, &output_path, cache::PREVIEW_LIMITS);
        return Ok(output_path);
    }

    let staged_path =
        cache::staging_path(&output_path).map_err(|error| MediaError::Cache(error.to_string()))?;
    let source_text = source.to_string_lossy().into_owned();
    let output_text = staged_path.to_string_lossy().into_owned();
    let output = match app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|error| MediaError::ProcessStart {
            tool: "ffmpeg",
            message: error.to_string(),
        })?
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            &source_text,
            "-map",
            "0:v:0",
            "-an",
            "-vf",
            "scale=w='min(1280,iw)':h=-2:force_original_aspect_ratio=decrease,setsar=1",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "24",
            "-g",
            "30",
            "-keyint_min",
            "30",
            "-sc_threshold",
            "0",
            "-pix_fmt",
            "yuv420p",
            "-fps_mode",
            "passthrough",
            "-map_metadata",
            "-1",
            "-movflags",
            "+faststart",
            &output_text,
        ])
        .output()
        .await
    {
        Ok(output) => output,
        Err(error) => {
            remove_partial_cache_output(&staged_path);
            return Err(MediaError::ProcessStart {
                tool: "ffmpeg",
                message: error.to_string(),
            });
        }
    };

    if !output.status.success() {
        remove_partial_cache_output(&staged_path);
        return Err(MediaError::ProcessFailed {
            tool: "ffmpeg preview",
            message: compact_stderr(&output.stderr),
        });
    }

    cache::commit(&staged_path, &output_path)
        .map_err(|error| MediaError::Cache(error.to_string()))?;
    prune_cache(&cache_root, &output_path, cache::PREVIEW_LIMITS);
    Ok(output_path)
}

pub async fn create_timeline_strip(
    app: &AppHandle,
    source: &Path,
    duration_seconds: f64,
) -> Result<PathBuf, MediaError> {
    let cache_root = app
        .path()
        .app_cache_dir()
        .map_err(|error| MediaError::Cache(error.to_string()))?
        .join("timelines");
    fs::create_dir_all(&cache_root).map_err(|error| MediaError::Cache(error.to_string()))?;

    let output_path = cache_root.join(format!("{:016x}.jpg", source_cache_key(source)?));
    if cache::reusable_entry(&output_path).map_err(|error| MediaError::Cache(error.to_string()))? {
        prune_cache(&cache_root, &output_path, cache::TIMELINE_LIMITS);
        return Ok(output_path);
    }

    let duration = if duration_seconds.is_finite() && duration_seconds > 0.0 {
        duration_seconds
    } else {
        1.0
    };
    let sample_rate = 12.0 / duration;
    let filter = format!(
        "fps={sample_rate:.9},scale=160:90:force_original_aspect_ratio=increase,crop=160:90,tile=12x1:nb_frames=12"
    );
    let staged_path =
        cache::staging_path(&output_path).map_err(|error| MediaError::Cache(error.to_string()))?;
    let source_text = source.to_string_lossy().into_owned();
    let output_text = staged_path.to_string_lossy().into_owned();
    let output = match app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|error| MediaError::ProcessStart {
            tool: "ffmpeg",
            message: error.to_string(),
        })?
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            &source_text,
            "-map",
            "0:v:0",
            "-an",
            "-vf",
            &filter,
            "-frames:v",
            "1",
            "-q:v",
            "5",
            "-update",
            "1",
            &output_text,
        ])
        .output()
        .await
    {
        Ok(output) => output,
        Err(error) => {
            remove_partial_cache_output(&staged_path);
            return Err(MediaError::ProcessStart {
                tool: "ffmpeg timeline",
                message: error.to_string(),
            });
        }
    };

    if !output.status.success() {
        remove_partial_cache_output(&staged_path);
        return Err(MediaError::ProcessFailed {
            tool: "ffmpeg timeline",
            message: compact_stderr(&output.stderr),
        });
    }
    cache::commit(&staged_path, &output_path)
        .map_err(|error| MediaError::Cache(error.to_string()))?;
    prune_cache(&cache_root, &output_path, cache::TIMELINE_LIMITS);
    Ok(output_path)
}

fn prune_cache(root: &Path, retained_path: &Path, limits: cache::CacheLimits) {
    if let Err(error) = cache::prune(root, retained_path, limits) {
        log::warn!("unable to prune media cache {}: {error}", root.display());
    }
}

fn remove_partial_cache_output(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => log::warn!(
            "unable to remove partial cache output {}: {error}",
            path.display()
        ),
    }
}

fn descriptor_from_probe(
    source: &Path,
    document: ProbeDocument,
) -> Result<MediaDescriptor, MediaError> {
    let video = document
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("video"))
        .ok_or(MediaError::MissingVideo)?;
    let audio = document
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"));

    let coded_width = video.width.ok_or(MediaError::MissingVideoWidth)?;
    let coded_height = video.height.ok_or(MediaError::MissingVideoHeight)?;
    let rotation = normalized_rotation(video);
    let (display_width, display_height) = if rotation == 90 || rotation == 270 {
        (coded_height, coded_width)
    } else {
        (coded_width, coded_height)
    };
    let duration_seconds = video
        .duration
        .as_deref()
        .and_then(parse_positive)
        .or_else(|| {
            document
                .format
                .as_ref()
                .and_then(|format| format.duration.as_deref())
                .and_then(parse_positive)
        })
        .ok_or(MediaError::MissingVideoDuration)?;
    let frame_count = video
        .nb_read_frames
        .as_deref()
        .or(video.nb_frames.as_deref())
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .ok_or(MediaError::MissingVideoFrameCount)?;
    let frame_rate = preferred_frame_rate(video).ok_or(MediaError::MissingVideoFrameRate)?;
    let frame_seek_supported = supports_frame_seek(video, duration_seconds, frame_count);
    let video_codec = video.codec_name.clone().unwrap_or_else(|| "unknown".into());
    let pixel_format = video.pix_fmt.clone().unwrap_or_else(|| "unknown".into());
    let file_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| MediaError::InvalidSource(display_path(source)))?;

    Ok(MediaDescriptor {
        source_path: display_path(source),
        file_name,
        duration_seconds,
        frame_count,
        coded_width,
        coded_height,
        display_width,
        display_height,
        rotation_degrees: rotation,
        frame_rate,
        frame_seek_supported,
        video_codec: video_codec.clone(),
        pixel_format: pixel_format.clone(),
        bit_depth: parse_bit_depth(video, &pixel_format),
        has_audio: audio.is_some(),
        audio_codec: audio.and_then(|stream| stream.codec_name.clone()),
        color: ColorDescriptor {
            primaries: video.color_primaries.clone(),
            transfer: video.color_transfer.clone(),
            matrix: video.color_space.clone(),
            range: video.color_range.clone(),
        },
        metadata_crop_supported: video_codec == "h264" || video_codec == "hevc",
    })
}

fn supports_frame_seek(stream: &ProbeStream, duration_seconds: f64, frame_count: u64) -> bool {
    let Some(average_rate) = stream.avg_frame_rate.as_deref().and_then(parse_frame_rate) else {
        return false;
    };
    let Some(real_rate) = stream.r_frame_rate.as_deref().and_then(parse_frame_rate) else {
        return false;
    };
    if (average_rate - real_rate).abs() > average_rate.max(real_rate) * 1e-6 {
        return false;
    }
    let counted_duration = frame_count as f64 / average_rate;
    let tolerance = (2.0 / average_rate).max(0.05);
    (counted_duration - duration_seconds).abs() <= tolerance
}

fn normalized_rotation(stream: &ProbeStream) -> i32 {
    let raw = stream
        .side_data_list
        .iter()
        .find_map(|item| item.rotation)
        .or_else(|| {
            stream
                .tags
                .as_ref()
                .and_then(|tags| tags.rotate.as_deref())
                .and_then(|value| value.parse::<f64>().ok())
        })
        .unwrap_or(0.0);
    let rounded = raw.round() as i32;
    ((rounded % 360) + 360) % 360
}

fn preferred_frame_rate(stream: &ProbeStream) -> Option<String> {
    stream
        .avg_frame_rate
        .as_deref()
        .filter(|value| parse_frame_rate(value).is_some())
        .or(stream.r_frame_rate.as_deref())
        .filter(|value| parse_frame_rate(value).is_some())
        .map(str::to_owned)
}

fn parse_bit_depth(stream: &ProbeStream, pixel_format: &str) -> Option<u8> {
    stream
        .bits_per_raw_sample
        .as_deref()
        .and_then(|value| value.parse::<u8>().ok())
        .or_else(|| {
            [16_u8, 14, 12, 10, 9]
                .into_iter()
                .find(|depth| pixel_format.contains(&depth.to_string()))
        })
}

fn parse_positive(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|parsed| parsed.is_finite() && *parsed > 0.0)
}

fn parse_frame_rate(value: &str) -> Option<f64> {
    let mut parts = value.split('/');
    let numerator = parts.next()?.parse::<f64>().ok()?;
    let denominator = parts.next().unwrap_or("1").parse::<f64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let rate = numerator / denominator;
    (rate.is_finite() && rate > 0.0).then_some(rate)
}

fn source_cache_key(source: &Path) -> Result<u64, MediaError> {
    let metadata = source
        .metadata()
        .map_err(|error| MediaError::InvalidSource(error.to_string()))?;
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    match metadata.modified().and_then(|value| {
        value
            .duration_since(UNIX_EPOCH)
            .map_err(std::io::Error::other)
    }) {
        Ok(value) => value.as_nanos().hash(&mut hasher),
        Err(error) => {
            log::warn!(
                "source modification time is unavailable for {}; cache reuse is disabled: {error}",
                source.display()
            );
            uuid::Uuid::new_v4().hash(&mut hasher);
        }
    }
    Ok(hasher.finish())
}

fn compact_stderr(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    text.lines().take(8).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{ProbeFormat, ProbeSideData, ProbeTags};

    fn video_stream() -> ProbeStream {
        ProbeStream {
            codec_type: Some("video".into()),
            codec_name: Some("h264".into()),
            width: Some(1920),
            height: Some(1080),
            duration: Some("10.5".into()),
            avg_frame_rate: Some("30000/1001".into()),
            r_frame_rate: Some("30000/1001".into()),
            nb_frames: Some("314".into()),
            nb_read_frames: Some("315".into()),
            pix_fmt: Some("yuv420p".into()),
            bits_per_raw_sample: Some("8".into()),
            color_primaries: Some("bt709".into()),
            color_transfer: Some("bt709".into()),
            color_space: Some("bt709".into()),
            color_range: Some("tv".into()),
            tags: Some(ProbeTags { rotate: None }),
            side_data_list: vec![],
        }
    }

    #[test]
    fn rotates_display_dimensions_for_portrait_media() {
        let mut video = video_stream();
        video.side_data_list = vec![ProbeSideData {
            rotation: Some(-90.0),
        }];
        let document = ProbeDocument {
            streams: vec![video],
            format: Some(ProbeFormat { duration: None }),
        };

        let descriptor = descriptor_from_probe(Path::new("phone.mp4"), document).unwrap();

        assert_eq!(descriptor.rotation_degrees, 270);
        assert_eq!(
            (descriptor.display_width, descriptor.display_height),
            (1080, 1920)
        );
    }

    #[test]
    fn falls_back_to_format_duration() {
        let mut video = video_stream();
        video.duration = None;
        let document = ProbeDocument {
            streams: vec![video],
            format: Some(ProbeFormat {
                duration: Some("7.25".into()),
            }),
        };

        let descriptor = descriptor_from_probe(Path::new("clip.mp4"), document).unwrap();

        assert_eq!(descriptor.duration_seconds, 7.25);
        assert_eq!(descriptor.frame_count, 315);
        assert!(descriptor.metadata_crop_supported);
    }

    #[test]
    fn reuses_only_an_unchanged_current_source_probe() {
        let source = Path::new("clip.mp4");
        let descriptor = descriptor_from_probe(
            source,
            ProbeDocument {
                streams: vec![video_stream()],
                format: None,
            },
        )
        .unwrap();
        let fingerprint = SourceFingerprint {
            length: 1024,
            modified: UNIX_EPOCH,
        };
        let cache = ProbeCache::default();
        cache.store(source, fingerprint.clone(), &descriptor);

        assert_eq!(
            cache
                .get(source, &fingerprint)
                .map(|cached| cached.frame_count),
            Some(descriptor.frame_count)
        );
        assert!(
            cache
                .get(
                    source,
                    &SourceFingerprint {
                        length: 2048,
                        modified: UNIX_EPOCH,
                    },
                )
                .is_none()
        );
        assert!(cache.get(Path::new("other.mp4"), &fingerprint).is_none());
        cache.invalidate(source);
        assert!(cache.get(source, &fingerprint).is_none());
    }

    #[test]
    fn enables_frame_seeking_only_for_consistent_constant_frame_rate_timing() {
        let constant = descriptor_from_probe(
            Path::new("constant.mp4"),
            ProbeDocument {
                streams: vec![video_stream()],
                format: None,
            },
        )
        .unwrap();
        assert!(constant.frame_seek_supported);

        let mut variable_stream = video_stream();
        variable_stream.r_frame_rate = Some("60/1".into());
        let variable = descriptor_from_probe(
            Path::new("variable.mp4"),
            ProbeDocument {
                streams: vec![variable_stream],
                format: None,
            },
        )
        .unwrap();
        assert!(!variable.frame_seek_supported);
    }

    #[test]
    fn rejects_probe_data_without_exact_timing_information() {
        let mut video = video_stream();
        video.duration = None;
        video.nb_frames = None;
        video.nb_read_frames = None;
        video.avg_frame_rate = Some("0/0".into());
        video.r_frame_rate = Some("0/0".into());

        let no_duration = descriptor_from_probe(
            Path::new("clip.mp4"),
            ProbeDocument {
                streams: vec![video.clone()],
                format: Some(ProbeFormat { duration: None }),
            },
        );
        assert!(matches!(no_duration, Err(MediaError::MissingVideoDuration)));

        video.duration = Some("10".into());
        let no_count = descriptor_from_probe(
            Path::new("clip.mp4"),
            ProbeDocument {
                streams: vec![video.clone()],
                format: None,
            },
        );
        assert!(matches!(no_count, Err(MediaError::MissingVideoFrameCount)));

        video.nb_read_frames = Some("300".into());
        let no_rate = descriptor_from_probe(
            Path::new("clip.mp4"),
            ProbeDocument {
                streams: vec![video],
                format: None,
            },
        );
        assert!(matches!(no_rate, Err(MediaError::MissingVideoFrameRate)));
        assert_eq!(parse_frame_rate("30/1/2"), None);
    }

    #[cfg(windows)]
    #[test]
    fn removes_windows_extended_prefix_for_display() {
        assert_eq!(
            display_path(Path::new(r"\\?\C:\Users\Example\clip.mp4")),
            r"C:\Users\Example\clip.mp4"
        );
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\server\videos\clip.mp4")),
            r"\\server\videos\clip.mp4"
        );
    }
}
