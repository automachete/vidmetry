use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use crate::media::{ColorDescriptor, MediaDescriptor, ProbeDocument, ProbeStream};
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
    Cache(String),
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

pub async fn probe(app: &AppHandle, source: &Path) -> Result<MediaDescriptor, MediaError> {
    let source_text = source.to_string_lossy().into_owned();
    let output = app
        .shell()
        .sidecar("ffprobe")
        .map_err(|error| MediaError::ProcessStart {
            tool: "ffprobe",
            message: error.to_string(),
        })?
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
            &source_text,
        ])
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

    let document: ProbeDocument = serde_json::from_slice(&output.stdout)
        .map_err(|error| MediaError::InvalidProbe(error.to_string()))?;
    descriptor_from_probe(source, document)
}

pub async fn create_preview(app: &AppHandle, source: &Path) -> Result<PathBuf, MediaError> {
    let cache_root = app
        .path()
        .app_cache_dir()
        .map_err(|error| MediaError::Cache(error.to_string()))?
        .join("previews");
    fs::create_dir_all(&cache_root).map_err(|error| MediaError::Cache(error.to_string()))?;

    let output_path = cache_root.join(format!("{:016x}.mp4", source_cache_key(source)?));
    if output_path.is_file()
        && output_path
            .metadata()
            .map(|item| item.len() > 0)
            .unwrap_or(false)
    {
        return Ok(output_path);
    }

    let source_text = source.to_string_lossy().into_owned();
    let output_text = output_path.to_string_lossy().into_owned();
    let output = app
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
        .map_err(|error| MediaError::ProcessStart {
            tool: "ffmpeg",
            message: error.to_string(),
        })?;

    if !output.status.success() {
        let _ = fs::remove_file(&output_path);
        return Err(MediaError::ProcessFailed {
            tool: "ffmpeg preview",
            message: compact_stderr(&output.stderr),
        });
    }

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
    if output_path
        .metadata()
        .map(|item| item.len() > 0)
        .unwrap_or(false)
    {
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
    let source_text = source.to_string_lossy().into_owned();
    let output_text = output_path.to_string_lossy().into_owned();
    let output = app
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
        .map_err(|error| MediaError::ProcessStart {
            tool: "ffmpeg timeline",
            message: error.to_string(),
        })?;

    if !output.status.success() {
        let _ = fs::remove_file(&output_path);
        return Err(MediaError::ProcessFailed {
            tool: "ffmpeg timeline",
            message: compact_stderr(&output.stderr),
        });
    }
    Ok(output_path)
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
        .and_then(parse_nonnegative)
        .or_else(|| {
            document
                .format
                .as_ref()
                .and_then(|format| format.duration.as_deref())
                .and_then(parse_nonnegative)
        })
        .unwrap_or(0.0);
    let video_codec = video.codec_name.clone().unwrap_or_else(|| "unknown".into());
    let pixel_format = video.pix_fmt.clone().unwrap_or_else(|| "unknown".into());

    Ok(MediaDescriptor {
        source_path: display_path(source),
        file_name: source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "video".into()),
        duration_seconds,
        frame_count: video
            .nb_frames
            .as_deref()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0),
        coded_width,
        coded_height,
        display_width,
        display_height,
        rotation_degrees: rotation,
        sample_aspect_ratio: video
            .sample_aspect_ratio
            .clone()
            .unwrap_or_else(|| "1:1".into()),
        frame_rate: preferred_frame_rate(video),
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

fn preferred_frame_rate(stream: &ProbeStream) -> String {
    stream
        .avg_frame_rate
        .as_deref()
        .filter(|value| *value != "0/0")
        .or(stream.r_frame_rate.as_deref())
        .unwrap_or("0/0")
        .to_owned()
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
        .or(Some(8))
}

fn parse_nonnegative(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|parsed| parsed.is_finite() && *parsed >= 0.0)
}

fn source_cache_key(source: &Path) -> Result<u64, MediaError> {
    let metadata = source
        .metadata()
        .map_err(|error| MediaError::InvalidSource(error.to_string()))?;
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .hash(&mut hasher);
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
            nb_frames: Some("315".into()),
            sample_aspect_ratio: Some("1:1".into()),
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
        assert_eq!(descriptor.frame_count, Some(315));
        assert!(descriptor.metadata_crop_supported);
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
