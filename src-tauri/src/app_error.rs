use serde::Serialize;

use crate::ffmpeg::MediaError;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    SelectedVideoUnavailable,
    MediaProcessStartFailed,
    MediaProcessFailed,
    InvalidMediaInformation,
    VideoStreamMissing,
    VideoWidthMissing,
    VideoHeightMissing,
    PreviewCacheUnavailable,
    PreviewAuthorizationFailed,
    SelectedPathUnavailable,
    SelectedPathUnsupported,
    FolderReadFailed,
    FolderContainsNoSupportedVideos,
    ExportProcessPrepareFailed,
    ExportProcessStartFailed,
    ExportStateUpdateFailed,
    ExportCancelled,
    ExportProcessFailed,
    ExportStateReadFailed,
    CancellationStateUpdateFailed,
    ExportProcessStopFailed,
    CropTooSmall,
    CropEvenValuesRequired,
    CropInvalid,
    CropOutsideVideo,
    TrimOutsideVideo,
    MetadataTrimUnsupported,
    MetadataCodecUnsupported,
    CrfOutOfRange,
    AudioBitrateOutOfRange,
    FrameRateOutOfRange,
    CompatibleAudioUnsupported,
    DestinationMustBeAbsolute,
    DestinationFolderUnavailable,
    DestinationFolderMissing,
    DestinationFileNameMissing,
    DestinationFileInspectionFailed,
    SourceReplacementRequiresSave,
    DestinationAlreadyExists,
    SaveDestinationMismatch,
    CompatibleExtensionRequired,
    LosslessExtensionRequired,
    MetadataExtensionUnsupported,
    DestinationExtensionMissing,
    CommitOutputFailed,
    ExplorerOpenFailed,
    #[cfg(not(windows))]
    ExplorerUnsupported,
    AccentColorUnavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub const fn new(code: ErrorCode) -> Self {
        Self { code, detail: None }
    }

    pub fn with_detail(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: Some(detail.into()),
        }
    }
}

impl From<MediaError> for AppError {
    fn from(error: MediaError) -> Self {
        match error {
            MediaError::InvalidSource(detail) => {
                Self::with_detail(ErrorCode::SelectedVideoUnavailable, detail)
            }
            MediaError::ProcessStart { tool, message } => Self::with_detail(
                ErrorCode::MediaProcessStartFailed,
                format!("{tool}: {message}"),
            ),
            MediaError::ProcessFailed { tool, message } => {
                Self::with_detail(ErrorCode::MediaProcessFailed, format!("{tool}: {message}"))
            }
            MediaError::InvalidProbe(detail) => {
                Self::with_detail(ErrorCode::InvalidMediaInformation, detail)
            }
            MediaError::MissingVideo => Self::new(ErrorCode::VideoStreamMissing),
            MediaError::MissingVideoWidth => Self::new(ErrorCode::VideoWidthMissing),
            MediaError::MissingVideoHeight => Self::new(ErrorCode::VideoHeightMissing),
            MediaError::Cache(detail) => {
                Self::with_detail(ErrorCode::PreviewCacheUnavailable, detail)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_a_stable_language_neutral_contract() {
        let error = AppError::with_detail(ErrorCode::CropOutsideVideo, "1921 > 1920");
        let value = serde_json::to_value(error).expect("serialize app error");

        assert_eq!(value["code"], "crop_outside_video");
        assert_eq!(value["detail"], "1921 > 1920");
    }

    #[test]
    fn omits_an_absent_detail() {
        let value = serde_json::to_value(AppError::new(ErrorCode::VideoStreamMissing))
            .expect("serialize app error");

        assert_eq!(value, serde_json::json!({ "code": "video_stream_missing" }));
    }
}
