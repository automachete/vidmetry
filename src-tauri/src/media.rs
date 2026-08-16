use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDescriptor {
    pub source_path: String,
    pub file_name: String,
    pub duration_seconds: f64,
    pub frame_count: u64,
    pub coded_width: u32,
    pub coded_height: u32,
    pub display_width: u32,
    pub display_height: u32,
    pub rotation_degrees: i32,
    pub frame_rate: String,
    pub video_codec: String,
    pub pixel_format: String,
    pub bit_depth: Option<u8>,
    pub has_audio: bool,
    pub audio_codec: Option<String>,
    #[serde(skip_serializing)]
    pub color: ColorDescriptor,
    pub metadata_crop_supported: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorDescriptor {
    pub primaries: Option<String>,
    pub transfer: Option<String>,
    pub matrix: Option<String>,
    pub range: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProbeDocument {
    #[serde(default)]
    pub streams: Vec<ProbeStream>,
    pub format: Option<ProbeFormat>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProbeFormat {
    pub duration: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProbeStream {
    pub codec_type: Option<String>,
    pub codec_name: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration: Option<String>,
    pub avg_frame_rate: Option<String>,
    pub r_frame_rate: Option<String>,
    pub nb_frames: Option<String>,
    pub nb_read_frames: Option<String>,
    pub pix_fmt: Option<String>,
    pub bits_per_raw_sample: Option<String>,
    pub color_primaries: Option<String>,
    pub color_transfer: Option<String>,
    pub color_space: Option<String>,
    pub color_range: Option<String>,
    pub tags: Option<ProbeTags>,
    #[serde(default)]
    pub side_data_list: Vec<ProbeSideData>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProbeTags {
    pub rotate: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProbeSideData {
    pub rotation: Option<f64>,
}
