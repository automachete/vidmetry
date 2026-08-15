mod export;
mod ffmpeg;
mod media;
mod selection;

use tauri::Manager;

#[tauri::command]
fn health_check() -> &'static str {
    "Vidmetry media service is ready"
}

#[tauri::command]
async fn probe_video(
    app: tauri::AppHandle,
    path: String,
) -> Result<media::MediaDescriptor, String> {
    let source = ffmpeg::canonical_source(&path).map_err(|error| error.to_string())?;
    let descriptor = ffmpeg::probe(&app, &source)
        .await
        .map_err(|error| error.to_string())?;
    app.asset_protocol_scope()
        .allow_file(&source)
        .map_err(|error| format!("Unable to authorize video preview: {error}"))?;
    Ok(descriptor)
}

#[tauri::command]
async fn create_preview(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let source = ffmpeg::canonical_source(&path).map_err(|error| error.to_string())?;
    let preview = ffmpeg::create_preview(&app, &source)
        .await
        .map_err(|error| error.to_string())?;
    app.asset_protocol_scope()
        .allow_file(&preview)
        .map_err(|error| format!("Unable to authorize proxy preview: {error}"))?;
    Ok(preview.to_string_lossy().into_owned())
}

#[tauri::command]
async fn start_export(
    app: tauri::AppHandle,
    state: tauri::State<'_, export::ExportState>,
    request: export::ExportRequest,
) -> Result<String, String> {
    export::start(app, state, request).await
}

#[tauri::command]
fn cancel_export(
    state: tauri::State<'_, export::ExportState>,
    job_id: String,
) -> Result<(), String> {
    export::cancel(state, job_id)
}

#[tauri::command]
fn inspect_selection(path: String) -> Result<selection::SelectionDescriptor, String> {
    selection::inspect(&path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(export::ExportState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            health_check,
            probe_video,
            create_preview,
            start_export,
            cancel_export,
            inspect_selection
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Vidmetry");
}
