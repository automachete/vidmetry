use std::{ffi::OsString, path::PathBuf};

use crate::app_error::{AppError, ErrorCode};

const DIRECTORY_VERB_KEY: &str = r"Software\Classes\Directory\shell\Vidmetry";
const STATE_KEY: &str = r"Software\Vidmetry";
pub(crate) const PACKAGED_STATE_VALUE: &str = "ExplorerIntegrationEnabled";
const NSIS_STATE_VALUE: &str = "NsisExplorerIntegrationEnabled";

#[derive(Debug, PartialEq, Eq)]
enum RegistryAction {
    SetString {
        key: String,
        name: String,
        value: String,
    },
    SetDword {
        key: String,
        name: String,
        value: u32,
    },
    DeleteTree {
        key: String,
    },
}

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
        let plan = if is_packaged() {
            packaged_visibility_plan(enabled)
        } else {
            ensure_nsis_installation().map_err(integration_error)?;
            let executable = std::env::current_exe().map_err(integration_error)?;
            unpackaged_visibility_plan(&executable, enabled)
        };
        apply_registry_plan(&plan).map_err(integration_error)?;
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

fn packaged_visibility_plan(enabled: bool) -> Vec<RegistryAction> {
    vec![RegistryAction::SetDword {
        key: STATE_KEY.into(),
        name: PACKAGED_STATE_VALUE.into(),
        value: u32::from(enabled),
    }]
}

fn unpackaged_visibility_plan(executable: &std::path::Path, enabled: bool) -> Vec<RegistryAction> {
    let state = RegistryAction::SetDword {
        key: STATE_KEY.into(),
        name: NSIS_STATE_VALUE.into(),
        value: u32::from(enabled),
    };
    if !enabled {
        return vec![
            RegistryAction::DeleteTree {
                key: DIRECTORY_VERB_KEY.into(),
            },
            state,
        ];
    }

    let executable = executable.to_string_lossy();
    let command = format!(r#""{executable}" "%1""#);
    let icon = format!(r#"{executable},0"#);
    vec![
        set_string(DIRECTORY_VERB_KEY, "", "Open with Vidmetry"),
        set_string(DIRECTORY_VERB_KEY, "Icon", &icon),
        set_string(DIRECTORY_VERB_KEY, "MultiSelectModel", "Single"),
        set_string(&format!(r"{DIRECTORY_VERB_KEY}\command"), "", &command),
        state,
    ]
}

fn set_string(key: &str, name: &str, value: &str) -> RegistryAction {
    RegistryAction::SetString {
        key: key.into(),
        name: name.into(),
        value: value.into(),
    }
}

fn integration_error(error: impl std::fmt::Display) -> AppError {
    AppError::with_detail(
        ErrorCode::ExplorerIntegrationUpdateFailed,
        error.to_string(),
    )
}

#[cfg(windows)]
fn apply_registry_plan(actions: &[RegistryAction]) -> std::io::Result<()> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    for action in actions {
        match action {
            RegistryAction::SetString { key, name, value } => {
                current_user.create_subkey(key)?.0.set_value(name, value)?;
            }
            RegistryAction::SetDword { key, name, value } => {
                current_user.create_subkey(key)?.0.set_value(name, value)?;
            }
            RegistryAction::DeleteTree { key } => {
                if let Err(error) = current_user.delete_subkey_all(key)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(error);
                }
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn ensure_nsis_installation() -> std::io::Result<()> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    let state = RegKey::predef(HKEY_CURRENT_USER).open_subkey(STATE_KEY)?;
    let _: u32 = state.get_value(NSIS_STATE_VALUE)?;
    Ok(())
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

    #[test]
    fn packaged_visibility_changes_only_the_shared_state() {
        assert_eq!(
            packaged_visibility_plan(false),
            vec![RegistryAction::SetDword {
                key: STATE_KEY.into(),
                name: PACKAGED_STATE_VALUE.into(),
                value: 0,
            }]
        );
    }

    #[test]
    fn unpackaged_visibility_registers_the_same_directory_command_as_msix() {
        let plan = unpackaged_visibility_plan(
            std::path::Path::new(r"C:\Program Files\Vidmetry\vidmetry.exe"),
            true,
        );

        assert!(plan.contains(&set_string(DIRECTORY_VERB_KEY, "", "Open with Vidmetry")));
        assert!(plan.contains(&set_string(
            &format!(r"{DIRECTORY_VERB_KEY}\command"),
            "",
            r#""C:\Program Files\Vidmetry\vidmetry.exe" "%1""#
        )));
        assert!(plan.contains(&set_string(
            DIRECTORY_VERB_KEY,
            "MultiSelectModel",
            "Single"
        )));
        assert_eq!(
            plan.last(),
            Some(&RegistryAction::SetDword {
                key: STATE_KEY.into(),
                name: NSIS_STATE_VALUE.into(),
                value: 1,
            })
        );
    }

    #[test]
    fn disabling_unpackaged_visibility_keeps_video_open_with_registration() {
        let plan = unpackaged_visibility_plan(std::path::Path::new("vidmetry.exe"), false);

        assert_eq!(
            plan,
            vec![
                RegistryAction::DeleteTree {
                    key: DIRECTORY_VERB_KEY.into(),
                },
                RegistryAction::SetDword {
                    key: STATE_KEY.into(),
                    name: NSIS_STATE_VALUE.into(),
                    value: 0,
                },
            ]
        );
    }
}
