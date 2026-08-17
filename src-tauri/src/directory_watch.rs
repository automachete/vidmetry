use std::{fs, path::Path, sync::Mutex};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::{
    app_error::{AppError, ErrorCode},
    ffmpeg,
};

const DIRECTORY_CHANGED_EVENT: &str = "directory-changed";

#[derive(Default)]
pub struct DirectoryWatchState {
    watcher: Mutex<Option<RecommendedWatcher>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryChangedEvent {
    root_path: String,
}

#[tauri::command]
pub fn watch_directory(
    app: AppHandle,
    state: State<'_, DirectoryWatchState>,
    path: Option<String>,
) -> Result<(), AppError> {
    let mut active = state.watcher.lock().map_err(|error| {
        AppError::with_detail(ErrorCode::DirectoryWatchFailed, error.to_string())
    })?;

    let Some(path) = path else {
        *active = None;
        return Ok(());
    };

    let root = canonical_directory(&path)?;
    let root_path = ffmpeg::display_path(&root);
    let event_app = app.clone();
    let event_root_path = root_path.clone();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        match result {
            Ok(event) if should_refresh(&event.kind) => {
                if let Err(error) = event_app.emit(
                    DIRECTORY_CHANGED_EVENT,
                    DirectoryChangedEvent {
                        root_path: event_root_path.clone(),
                    },
                ) {
                    log::warn!("directory change event could not be emitted: {error}");
                }
            }
            Ok(_) => {}
            Err(error) => log::warn!("directory watcher reported an error: {error}"),
        }
    })
    .map_err(|error| AppError::with_detail(ErrorCode::DirectoryWatchFailed, error.to_string()))?;

    watcher
        .watch(&root, RecursiveMode::NonRecursive)
        .map_err(|error| {
            AppError::with_detail(ErrorCode::DirectoryWatchFailed, error.to_string())
        })?;
    *active = Some(watcher);
    Ok(())
}

fn canonical_directory(path: &str) -> Result<std::path::PathBuf, AppError> {
    let directory = fs::canonicalize(Path::new(path)).map_err(|error| {
        AppError::with_detail(ErrorCode::DirectoryWatchFailed, error.to_string())
    })?;
    if !directory.is_dir() {
        return Err(AppError::new(ErrorCode::DirectoryWatchFailed));
    }
    Ok(directory)
}

fn should_refresh(kind: &EventKind) -> bool {
    !matches!(kind, EventKind::Access(_))
}

#[cfg(test)]
mod tests {
    use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};

    use super::*;

    #[test]
    fn refreshes_for_directory_content_changes_but_not_file_access() {
        assert!(should_refresh(&EventKind::Create(CreateKind::File)));
        assert!(should_refresh(&EventKind::Modify(ModifyKind::Any)));
        assert!(should_refresh(&EventKind::Remove(RemoveKind::File)));
        assert!(!should_refresh(&EventKind::Access(AccessKind::Read)));
    }
}
