mod app_error;
mod appearance;
mod cache;
mod export;
mod ffmpeg;
mod media;
mod reveal;
mod selection;

use tauri::Manager;

use app_error::{AppError, ErrorCode};

#[tauri::command]
async fn probe_video(
    app: tauri::AppHandle,
    path: String,
) -> Result<media::MediaDescriptor, AppError> {
    let source = ffmpeg::canonical_source(&path).map_err(AppError::from)?;
    let descriptor = ffmpeg::probe(&app, &source).await.map_err(AppError::from)?;
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
    request: export::ExportRequest,
) -> Result<String, AppError> {
    export::start(app, state, request).await
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
fn reveal_in_explorer(path: String) -> Result<(), AppError> {
    reveal::in_explorer(&path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(export::ExportState::default())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            appearance::system_accent_color,
            probe_video,
            create_preview,
            create_timeline_strip,
            start_export,
            cancel_export,
            inspect_selection,
            reveal_in_explorer
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Vidmetry");
}
