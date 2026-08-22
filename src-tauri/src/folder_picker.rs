#[cfg(windows)]
mod windows_picker;

#[cfg(windows)]
pub fn pick(
    owner: isize,
    title: &str,
    select_folder_label: &str,
    cancel_label: &str,
    initial_directory: Option<&str>,
) -> Result<Option<String>, String> {
    windows_picker::pick(
        owner,
        title,
        select_folder_label,
        cancel_label,
        initial_directory,
    )
}

#[cfg(windows)]
pub fn windows_ui_language() -> &'static str {
    windows_picker::windows_ui_language()
}
