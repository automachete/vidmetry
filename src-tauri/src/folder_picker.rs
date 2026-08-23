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
pub fn pick(
    owner: isize,
    title: &str,
    select_folder_label: &str,
    cancel_label: &str,
    initial_directory: Option<&str>,
    initial_view_mode: Option<i32>,
    initial_icon_size: Option<i32>,
) -> Result<Option<FolderPickerSelection>, String> {
    windows_picker::pick(
        owner,
        title,
        select_folder_label,
        cancel_label,
        initial_directory,
        initial_view_mode,
        initial_icon_size,
    )
}

#[cfg(windows)]
pub fn windows_ui_language() -> &'static str {
    windows_picker::windows_ui_language()
}
