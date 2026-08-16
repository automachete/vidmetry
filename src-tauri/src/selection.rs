use std::{fs, path::PathBuf};

use serde::Serialize;

use crate::{
    app_error::{AppError, ErrorCode},
    ffmpeg,
};

const VIDEO_EXTENSIONS: &[&str] = &[
    "3gp", "avi", "flv", "m2ts", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "mts", "ogv", "ts",
    "vob", "webm", "wmv",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionDescriptor {
    pub kind: SelectionKind,
    pub root_path: String,
    pub video_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionKind {
    File,
    Directory,
}

pub fn inspect(path: &str) -> Result<SelectionDescriptor, AppError> {
    let selected = fs::canonicalize(path).map_err(|error| {
        AppError::with_detail(ErrorCode::SelectedPathUnavailable, error.to_string())
    })?;
    if selected.is_file() {
        let source = ffmpeg::canonical_source(path).map_err(AppError::from)?;
        return Ok(SelectionDescriptor {
            kind: SelectionKind::File,
            root_path: ffmpeg::display_path(&source),
            video_paths: vec![ffmpeg::display_path(&source)],
        });
    }
    if !selected.is_dir() {
        return Err(AppError::new(ErrorCode::SelectedPathUnsupported));
    }

    let mut videos = Vec::<PathBuf>::new();
    for entry in fs::read_dir(&selected)
        .map_err(|error| AppError::with_detail(ErrorCode::FolderReadFailed, error.to_string()))?
    {
        let entry = entry.map_err(|error| {
            AppError::with_detail(ErrorCode::FolderReadFailed, error.to_string())
        })?;
        let metadata = entry.metadata().map_err(|error| {
            AppError::with_detail(ErrorCode::FolderReadFailed, error.to_string())
        })?;
        let path = entry.path();
        if metadata.is_file() && is_video_path(&path) {
            videos.push(path);
        }
    }
    videos.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
            .cmp(
                &right
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase(),
            )
    });

    if videos.is_empty() {
        return Err(AppError::new(ErrorCode::FolderContainsNoSupportedVideos));
    }

    Ok(SelectionDescriptor {
        kind: SelectionKind::Directory,
        root_path: ffmpeg::display_path(&selected),
        video_paths: videos
            .into_iter()
            .map(|path| ffmpeg::display_path(&path))
            .collect(),
    })
}

fn is_video_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| VIDEO_EXTENSIONS.contains(&extension.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use uuid::Uuid;

    #[test]
    fn recognizes_supported_extensions_case_insensitively() {
        assert!(is_video_path(std::path::Path::new("clip.MP4")));
        assert!(is_video_path(std::path::Path::new("phone.m2ts")));
        assert!(!is_video_path(std::path::Path::new("notes.txt")));
    }

    #[test]
    fn inspects_a_directory_non_recursively_and_sorts_video_names() {
        let root = std::env::temp_dir().join(format!("vidmetry-selection-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join("nested")).expect("create fixture directories");
        File::create(root.join("z-last.MP4")).expect("create video fixture");
        File::create(root.join("A-first.mkv")).expect("create video fixture");
        File::create(root.join("notes.txt")).expect("create non-video fixture");
        File::create(root.join("nested").join("hidden.mov")).expect("create nested fixture");

        let result = inspect(root.to_str().expect("fixture path should be UTF-8"))
            .expect("directory inspection should succeed");
        let names = result
            .video_paths
            .iter()
            .map(|path| {
                std::path::Path::new(path)
                    .file_name()
                    .expect("file name")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();

        assert!(matches!(result.kind, SelectionKind::Directory));
        assert_eq!(names, vec!["A-first.mkv", "z-last.MP4"]);
        fs::remove_dir_all(root).expect("remove fixture directory");
    }
}
