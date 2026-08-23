#[cfg(windows)]
mod windows_picker;

#[cfg(windows)]
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderPickerSelection {
    pub path: String,
    pub view_mode: i32,
    pub icon_size: i32,
}

#[cfg(windows)]
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderPickerViewLabels {
    pub view: String,
    pub extra_large_icons: String,
    pub large_icons: String,
    pub medium_icons: String,
    pub small_icons: String,
    pub list: String,
    pub details: String,
    pub tiles: String,
    pub content: String,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderPickerViewSettings {
    pub view_mode: i32,
    pub icon_size: i32,
}

#[cfg(windows)]
pub fn pick(
    owner: isize,
    title: &str,
    select_folder_label: &str,
    cancel_label: &str,
    initial_directory: Option<&str>,
    initial_view: FolderPickerViewSettings,
    view_labels: FolderPickerViewLabels,
) -> Result<Option<FolderPickerSelection>, String> {
    windows_picker::pick(
        owner,
        title,
        select_folder_label,
        cancel_label,
        initial_directory,
        initial_view,
        view_labels,
    )
}

#[cfg(windows)]
pub fn windows_ui_language() -> &'static str {
    windows_picker::windows_ui_language()
}
