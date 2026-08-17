use std::{ffi::OsString, path::PathBuf};

use crate::{
    app_error::{AppError, ErrorCode},
    selection::VIDEO_EXTENSIONS,
};

const PROG_ID: &str = "Vidmetry.Video";
const APP_CLASSES_KEY: &str = r"Software\Classes\Applications\vidmetry.exe";
const PROG_ID_KEY: &str = r"Software\Classes\Vidmetry.Video";
const CAPABILITIES_KEY: &str = r"Software\Vidmetry\Capabilities";
const REGISTERED_APPLICATIONS_KEY: &str = r"Software\RegisteredApplications";
const DIRECTORY_VERB_KEY: &str = r"Software\Classes\Directory\shell\Vidmetry";
const STATE_KEY: &str = r"Software\Vidmetry";
pub(crate) const STATE_VALUE: &str = "ExplorerIntegrationEnabled";

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
    DeleteValue {
        key: String,
        name: String,
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
        let executable = std::env::current_exe().map_err(integration_error)?;
        apply_registry_plan(&registration_plan(&executable, enabled)).map_err(integration_error)?;
        notify_shell_association_changed();
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err(AppError::new(ErrorCode::ExplorerIntegrationUnsupported))
    }
}

fn registration_plan(executable: &std::path::Path, enabled: bool) -> Vec<RegistryAction> {
    if !enabled {
        let mut actions = VIDEO_EXTENSIONS
            .iter()
            .map(|extension| RegistryAction::DeleteValue {
                key: format!(r"Software\Classes\.{extension}\OpenWithProgids"),
                name: PROG_ID.into(),
            })
            .collect::<Vec<_>>();
        actions.extend([
            RegistryAction::DeleteTree {
                key: DIRECTORY_VERB_KEY.into(),
            },
            RegistryAction::DeleteTree {
                key: APP_CLASSES_KEY.into(),
            },
            RegistryAction::DeleteTree {
                key: PROG_ID_KEY.into(),
            },
            RegistryAction::DeleteTree {
                key: CAPABILITIES_KEY.into(),
            },
            RegistryAction::DeleteValue {
                key: REGISTERED_APPLICATIONS_KEY.into(),
                name: "Vidmetry".into(),
            },
            RegistryAction::SetDword {
                key: STATE_KEY.into(),
                name: STATE_VALUE.into(),
                value: 0,
            },
        ]);
        return actions;
    }

    let executable = executable.to_string_lossy();
    let command = format!(r#""{executable}" "%1""#);
    let icon = format!(r#"{executable},0"#);
    let mut actions = vec![
        set_string(PROG_ID_KEY, "", "Vidmetry video"),
        set_string(&format!(r"{PROG_ID_KEY}\DefaultIcon"), "", &icon),
        set_string(&format!(r"{PROG_ID_KEY}\shell\open\command"), "", &command),
        set_string(APP_CLASSES_KEY, "FriendlyAppName", "Vidmetry"),
        set_string(&format!(r"{APP_CLASSES_KEY}\DefaultIcon"), "", &icon),
        set_string(
            &format!(r"{APP_CLASSES_KEY}\shell\open\command"),
            "",
            &command,
        ),
        set_string(CAPABILITIES_KEY, "ApplicationName", "Vidmetry"),
        set_string(
            CAPABILITIES_KEY,
            "ApplicationDescription",
            "Crop and trim videos with Vidmetry.",
        ),
        set_string(
            REGISTERED_APPLICATIONS_KEY,
            "Vidmetry",
            r"Software\Vidmetry\Capabilities",
        ),
        set_string(DIRECTORY_VERB_KEY, "", "Open with Vidmetry"),
        set_string(DIRECTORY_VERB_KEY, "Icon", &icon),
        set_string(DIRECTORY_VERB_KEY, "MultiSelectModel", "Single"),
        set_string(&format!(r"{DIRECTORY_VERB_KEY}\command"), "", &command),
    ];

    for extension in VIDEO_EXTENSIONS {
        actions.push(set_string(
            &format!(r"Software\Classes\.{extension}\OpenWithProgids"),
            PROG_ID,
            "",
        ));
        actions.push(set_string(
            &format!(r"{APP_CLASSES_KEY}\SupportedTypes"),
            &format!(".{extension}"),
            "",
        ));
        actions.push(set_string(
            &format!(r"{CAPABILITIES_KEY}\FileAssociations"),
            &format!(".{extension}"),
            PROG_ID,
        ));
    }
    actions.push(RegistryAction::SetDword {
        key: STATE_KEY.into(),
        name: STATE_VALUE.into(),
        value: 1,
    });
    actions
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
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, KEY_SET_VALUE},
    };

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    for action in actions {
        match action {
            RegistryAction::SetString { key, name, value } => {
                current_user.create_subkey(key)?.0.set_value(name, value)?;
            }
            RegistryAction::SetDword { key, name, value } => {
                current_user.create_subkey(key)?.0.set_value(name, value)?;
            }
            RegistryAction::DeleteValue { key, name } => {
                let key = match current_user.open_subkey_with_flags(key, KEY_SET_VALUE) {
                    Ok(key) => key,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(error),
                };
                if let Err(error) = key.delete_value(name)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(error);
                }
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
    fn registration_covers_every_supported_video_and_selected_directories() {
        let plan = registration_plan(
            std::path::Path::new(r"C:\Program Files\Vidmetry\vidmetry.exe"),
            true,
        );

        for extension in VIDEO_EXTENSIONS {
            assert!(plan.contains(&set_string(
                &format!(r"Software\Classes\.{extension}\OpenWithProgids"),
                PROG_ID,
                ""
            )));
        }
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
                name: STATE_VALUE.into(),
                value: 1,
            })
        );
    }

    #[test]
    fn disabling_removes_only_vidmetry_owned_registrations() {
        let plan = registration_plan(std::path::Path::new("vidmetry.exe"), false);

        assert!(plan.contains(&RegistryAction::DeleteTree {
            key: DIRECTORY_VERB_KEY.into()
        }));
        assert!(plan.contains(&RegistryAction::DeleteValue {
            key: r"Software\Classes\.mp4\OpenWithProgids".into(),
            name: PROG_ID.into()
        }));
        assert!(!plan.iter().any(|action| matches!(
            action,
            RegistryAction::DeleteTree { key } if key.contains(r"Classes\.mp4")
        )));
        assert_eq!(
            plan.last(),
            Some(&RegistryAction::SetDword {
                key: STATE_KEY.into(),
                name: STATE_VALUE.into(),
                value: 0,
            })
        );
    }
}
