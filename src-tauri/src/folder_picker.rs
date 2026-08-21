#[cfg(windows)]
mod windows_picker {
    use std::sync::{Arc, Mutex};

    use windows::{
        Win32::{
            Foundation::{ERROR_CANCELLED, HWND},
            System::Com::{
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
            },
            UI::Shell::{
                Common::COMDLG_FILTERSPEC, FDE_OVERWRITE_RESPONSE, FDE_SHAREVIOLATION_RESPONSE,
                FDEOR_DEFAULT, FDESVR_DEFAULT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM,
                FOS_PATHMUSTEXIST, FileOpenDialog, IFileDialog, IFileDialogControlEvents,
                IFileDialogControlEvents_Impl, IFileDialogCustomize, IFileDialogEvents,
                IFileDialogEvents_Impl, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName,
                SIGDN_FILESYSPATH,
            },
        },
        core::{HRESULT, Interface, PCWSTR, Ref, Result as WindowsResult, implement},
    };

    use crate::selection::VIDEO_EXTENSIONS;

    const SELECT_FOLDER_BUTTON_ID: u32 = 0x4001;

    struct ComApartment;

    impl Drop for ComApartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    #[implement(IFileDialogEvents, IFileDialogControlEvents)]
    struct FolderDialogEvents {
        dialog: IFileDialog,
        selected_path: Arc<Mutex<Option<String>>>,
    }

    impl FolderDialogEvents {
        fn select_current_folder(&self) -> WindowsResult<()> {
            let folder = unsafe { self.dialog.GetFolder()? };
            let path = shell_item_path(&folder)?;
            *self
                .selected_path
                .lock()
                .expect("folder dialog selection lock") = Some(path);
            Ok(())
        }
    }

    impl IFileDialogEvents_Impl for FolderDialogEvents_Impl {
        fn OnFileOk(&self, _dialog: Ref<'_, IFileDialog>) -> WindowsResult<()> {
            self.select_current_folder()
        }

        fn OnFolderChanging(
            &self,
            _dialog: Ref<'_, IFileDialog>,
            _folder: Ref<'_, IShellItem>,
        ) -> WindowsResult<()> {
            Ok(())
        }

        fn OnFolderChange(&self, _dialog: Ref<'_, IFileDialog>) -> WindowsResult<()> {
            Ok(())
        }

        fn OnSelectionChange(&self, _dialog: Ref<'_, IFileDialog>) -> WindowsResult<()> {
            Ok(())
        }

        fn OnShareViolation(
            &self,
            _dialog: Ref<'_, IFileDialog>,
            _item: Ref<'_, IShellItem>,
        ) -> WindowsResult<FDE_SHAREVIOLATION_RESPONSE> {
            Ok(FDESVR_DEFAULT)
        }

        fn OnTypeChange(&self, _dialog: Ref<'_, IFileDialog>) -> WindowsResult<()> {
            Ok(())
        }

        fn OnOverwrite(
            &self,
            _dialog: Ref<'_, IFileDialog>,
            _item: Ref<'_, IShellItem>,
        ) -> WindowsResult<FDE_OVERWRITE_RESPONSE> {
            Ok(FDEOR_DEFAULT)
        }
    }

    impl IFileDialogControlEvents_Impl for FolderDialogEvents_Impl {
        fn OnItemSelected(
            &self,
            _dialog: Ref<'_, IFileDialogCustomize>,
            _control_id: u32,
            _item_id: u32,
        ) -> WindowsResult<()> {
            Ok(())
        }

        fn OnButtonClicked(
            &self,
            _dialog: Ref<'_, IFileDialogCustomize>,
            control_id: u32,
        ) -> WindowsResult<()> {
            if control_id == SELECT_FOLDER_BUTTON_ID {
                self.select_current_folder()?;
                unsafe { self.dialog.Close(HRESULT(0))? };
            }
            Ok(())
        }

        fn OnCheckButtonToggled(
            &self,
            _dialog: Ref<'_, IFileDialogCustomize>,
            _control_id: u32,
            _checked: windows::core::BOOL,
        ) -> WindowsResult<()> {
            Ok(())
        }

        fn OnControlActivating(
            &self,
            _dialog: Ref<'_, IFileDialogCustomize>,
            _control_id: u32,
        ) -> WindowsResult<()> {
            Ok(())
        }
    }

    pub(super) fn pick(
        owner: isize,
        title: &str,
        select_folder_label: &str,
        select_current_folder_label: &str,
        filter_name: &str,
        initial_directory: Option<&str>,
    ) -> Result<Option<String>, String> {
        let initialization =
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        initialization.ok().map_err(windows_error)?;
        let _com_apartment = ComApartment;

        let dialog: IFileOpenDialog = unsafe {
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).map_err(windows_error)?
        };
        let dialog: IFileDialog = dialog.cast().map_err(windows_error)?;
        let title = wide_string(title);
        let select_folder_label = wide_string(select_folder_label);
        let select_current_folder_label = wide_string(select_current_folder_label);
        unsafe {
            let options = dialog.GetOptions().map_err(windows_error)?;
            dialog
                .SetOptions(options | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_FILEMUSTEXIST)
                .map_err(windows_error)?;
            dialog
                .SetTitle(PCWSTR(title.as_ptr()))
                .map_err(windows_error)?;
            dialog
                .SetOkButtonLabel(PCWSTR(select_folder_label.as_ptr()))
                .map_err(windows_error)?;
        }

        let filter_name = wide_string(filter_name);
        let filter_pattern = supported_video_filter_pattern();
        let filter = [COMDLG_FILTERSPEC {
            pszName: PCWSTR(filter_name.as_ptr()),
            pszSpec: PCWSTR(filter_pattern.as_ptr()),
        }];
        unsafe { dialog.SetFileTypes(&filter).map_err(windows_error)? };

        if let Some(path) = initial_directory {
            let path = wide_string(path);
            let folder: IShellItem = unsafe {
                SHCreateItemFromParsingName(PCWSTR(path.as_ptr()), None).map_err(windows_error)?
            };
            unsafe { dialog.SetFolder(&folder).map_err(windows_error)? };
        }

        let customize: IFileDialogCustomize = dialog.cast().map_err(windows_error)?;
        unsafe {
            customize
                .AddPushButton(
                    SELECT_FOLDER_BUTTON_ID,
                    PCWSTR(select_current_folder_label.as_ptr()),
                )
                .map_err(windows_error)?;
            customize
                .MakeProminent(SELECT_FOLDER_BUTTON_ID)
                .map_err(windows_error)?;
        }

        let selected_path = Arc::new(Mutex::new(None));
        let events: IFileDialogEvents = FolderDialogEvents {
            dialog: dialog.clone(),
            selected_path: Arc::clone(&selected_path),
        }
        .into();
        let cookie = unsafe { dialog.Advise(&events).map_err(windows_error)? };
        let shown = unsafe { dialog.Show(Some(HWND(owner as *mut _))) };
        unsafe { dialog.Unadvise(cookie).map_err(windows_error)? };

        if let Err(error) = shown {
            if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Ok(None);
            }
            return Err(windows_error(error));
        }
        Ok(selected_path
            .lock()
            .expect("folder dialog selection lock")
            .clone())
    }

    fn supported_video_filter_pattern() -> Vec<u16> {
        wide_string(
            &VIDEO_EXTENSIONS
                .iter()
                .map(|extension| format!("*.{extension}"))
                .collect::<Vec<_>>()
                .join(";"),
        )
    }

    fn shell_item_path(item: &IShellItem) -> WindowsResult<String> {
        let path = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH)? };
        let result = unsafe { path.to_string() };
        unsafe { CoTaskMemFree(Some(path.0.cast())) };
        Ok(result?)
    }

    fn wide_string(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    fn windows_error(error: windows::core::Error) -> String {
        error.to_string()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn file_filter_contains_every_supported_video_extension() {
            let filter = String::from_utf16_lossy(&supported_video_filter_pattern());

            for extension in VIDEO_EXTENSIONS {
                assert!(filter.contains(&format!("*.{extension}")));
            }
        }
    }
}

#[cfg(windows)]
pub fn pick(
    owner: isize,
    title: &str,
    select_folder_label: &str,
    select_current_folder_label: &str,
    filter_name: &str,
    initial_directory: Option<&str>,
) -> Result<Option<String>, String> {
    windows_picker::pick(
        owner,
        title,
        select_folder_label,
        select_current_folder_label,
        filter_name,
        initial_directory,
    )
}
