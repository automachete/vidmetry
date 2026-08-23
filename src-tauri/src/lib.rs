pub mod app_error;
mod appearance;
mod cache;
mod directory_watch;
mod export;
mod ffmpeg;
mod folder_picker;
mod media;
mod reveal;
mod selection;
mod shell_integration;

use tauri::Manager;

use app_error::{AppError, ErrorCode};

#[tauri::command]
async fn probe_video(
    app: tauri::AppHandle,
    cache: tauri::State<'_, ffmpeg::ProbeCache>,
    path: String,
) -> Result<media::MediaDescriptor, AppError> {
    let source = ffmpeg::canonical_source(&path).map_err(AppError::from)?;
    let descriptor = ffmpeg::probe(&app, &cache, &source)
        .await
        .map_err(AppError::from)?;
    app.asset_protocol_scope()
        .allow_file(&source)
        .map_err(|error| {
            AppError::with_detail(ErrorCode::PreviewAuthorizationFailed, error.to_string())
        })?;
    Ok(descriptor)
}

#[tauri::command]
async fn create_preview(app: tauri::AppHandle, path: String) -> Result<String, AppError> {
    let source = ffmpeg::canonical_source(&path).map_err(AppError::from)?;
    let preview = ffmpeg::create_preview(&app, &source)
        .await
        .map_err(AppError::from)?;
    app.asset_protocol_scope()
        .allow_file(&preview)
        .map_err(|error| {
            AppError::with_detail(ErrorCode::PreviewAuthorizationFailed, error.to_string())
        })?;
    Ok(preview.to_string_lossy().into_owned())
}

#[tauri::command]
async fn create_timeline_strip(
    app: tauri::AppHandle,
    path: String,
    duration_seconds: f64,
) -> Result<String, AppError> {
    let source = ffmpeg::canonical_source(&path).map_err(AppError::from)?;
    let strip = ffmpeg::create_timeline_strip(&app, &source, duration_seconds)
        .await
        .map_err(AppError::from)?;
    app.asset_protocol_scope()
        .allow_file(&strip)
        .map_err(|error| {
            AppError::with_detail(ErrorCode::PreviewAuthorizationFailed, error.to_string())
        })?;
    Ok(strip.to_string_lossy().into_owned())
}

#[tauri::command]
async fn start_export(
    app: tauri::AppHandle,
    state: tauri::State<'_, export::ExportState>,
    probe_cache: tauri::State<'_, ffmpeg::ProbeCache>,
    request: export::ExportRequest,
) -> Result<String, AppError> {
    export::start(app, state, probe_cache, request).await
}

#[tauri::command]
async fn available_video_encoders(app: tauri::AppHandle) -> export::VideoEncoderAvailability {
    export::available_video_encoders(app).await
}

#[tauri::command]
fn cancel_export(
    state: tauri::State<'_, export::ExportState>,
    job_id: String,
) -> Result<(), AppError> {
    export::cancel(state, job_id)
}

#[tauri::command]
fn inspect_selection(path: String) -> Result<selection::SelectionDescriptor, AppError> {
    selection::inspect(&path)
}

#[tauri::command]
fn supported_video_extensions() -> Vec<&'static str> {
    selection::VIDEO_EXTENSIONS.to_vec()
}

#[tauri::command]
fn windows_ui_language() -> &'static str {
    folder_picker::windows_ui_language()
}

#[tauri::command]
async fn pick_video_folder(
    window: tauri::WebviewWindow,
    title: String,
    select_folder_label: String,
    cancel_label: String,
    initial_directory: Option<String>,
    initial_view_mode: Option<i32>,
    initial_icon_size: Option<i32>,
) -> Result<Option<folder_picker::FolderPickerSelection>, AppError> {
    let owner = window.hwnd().map_err(|error| {
        AppError::with_detail(ErrorCode::SelectedPathUnavailable, error.to_string())
    })?;
    let owner = owner.0 as isize;
    tauri::async_runtime::spawn_blocking(move || {
        folder_picker::pick(
            owner,
            &title,
            &select_folder_label,
            &cancel_label,
            initial_directory.as_deref(),
            initial_view_mode,
            initial_icon_size,
        )
        .map_err(|detail| AppError::with_detail(ErrorCode::SelectedPathUnavailable, detail))
    })
    .await
    .map_err(|error| AppError::with_detail(ErrorCode::SelectedPathUnavailable, error.to_string()))?
}

#[tauri::command]
fn reveal_in_explorer(path: String) -> Result<(), AppError> {
    reveal::in_explorer(&path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(export::ExportState::default())
        .manage(ffmpeg::ProbeCache::default())
        .manage(directory_watch::DirectoryWatchState::default())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            appearance::system_accent_color,
            probe_video,
            create_preview,
            create_timeline_strip,
            available_video_encoders,
            start_export,
            cancel_export,
            inspect_selection,
            supported_video_extensions,
            windows_ui_language,
            pick_video_folder,
            directory_watch::watch_directory,
            reveal_in_explorer,
            shell_integration::startup_selection,
            shell_integration::set_explorer_integration
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Vidmetry");
}
