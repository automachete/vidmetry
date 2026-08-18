use std::{ffi::OsString, path::PathBuf};

use crate::app_error::{AppError, ErrorCode};

const STATE_KEY: &str = r"Software\Vidmetry";
pub(crate) const STATE_VALUE: &str = "ExplorerIntegrationEnabled";

#[tauri::command]
pub fn startup_selection() -> Option<String> {
    startup_selection_from(std::env::args_os())
}

fn startup_selection_from(args: impl IntoIterator<Item = OsString>) -> Option<String> {
    args.into_iter().skip(1).find_map(|argument| {
        let path = PathBuf::from(argument);
        (path.is_file() || path.is_dir()).then(|| path.to_string_lossy().into_owned())
    })
}

#[tauri::command]
pub fn set_explorer_integration(enabled: bool) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        if !is_packaged() {
            return Err(AppError::with_detail(
                ErrorCode::ExplorerIntegrationUpdateFailed,
                "MSIX package identity is unavailable",
            ));
        }
        set_packaged_visibility(enabled).map_err(integration_error)?;
        notify_shell_association_changed();
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err(AppError::new(ErrorCode::ExplorerIntegrationUnsupported))
    }
}

#[cfg(windows)]
fn is_packaged() -> bool {
    use windows_sys::Win32::{
        Foundation::ERROR_INSUFFICIENT_BUFFER, Storage::Packaging::Appx::GetCurrentPackageFullName,
    };

    let mut length = 0;
    // SAFETY: A null buffer with a zero length is the documented package-identity probe.
    (unsafe { GetCurrentPackageFullName(&mut length, std::ptr::null_mut()) })
        == ERROR_INSUFFICIENT_BUFFER
}

#[cfg(windows)]
fn set_packaged_visibility(enabled: bool) -> std::io::Result<()> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(STATE_KEY)?
        .0
        .set_value(STATE_VALUE, &u32::from(enabled))
}

fn integration_error(error: impl std::fmt::Display) -> AppError {
    AppError::with_detail(
        ErrorCode::ExplorerIntegrationUpdateFailed,
        error.to_string(),
    )
}

#[cfg(windows)]
fn notify_shell_association_changed() {
    use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};

    // SAFETY: SHCNE_ASSOCCHANGED does not use item pointers; both are required to be null.
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as i32,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn finds_an_existing_file_or_directory_in_shell_arguments() {
        let root = std::env::temp_dir().join(format!("vidmetry-shell-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create test folder");

        assert_eq!(
            startup_selection_from([
                OsString::from("vidmetry.exe"),
                root.clone().into_os_string()
            ]),
            Some(root.to_string_lossy().into_owned())
        );

        fs::remove_dir_all(root).expect("remove test folder");
    }
}
