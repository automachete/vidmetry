use std::{
    ffi::c_void,
    path::Path,
    ptr::{null, null_mut},
    sync::Mutex,
};

use windows::{
    Win32::{
        Foundation::{
            ERROR_CANCELLED, ERROR_CLASS_ALREADY_EXISTS, GetLastError, HINSTANCE, HWND, LPARAM,
            LRESULT, RECT, S_FALSE, WPARAM,
        },
        Globalization::GetUserDefaultUILanguage,
        Graphics::Gdi::{
            COLOR_WINDOW, CreateFontIndirectW, DeleteObject, GetMonitorInfoW, HBRUSH, HFONT,
            MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, UpdateWindow,
        },
        System::{
            Com::{
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IServiceProvider,
                IServiceProvider_Impl,
            },
            SystemServices::{SFGAO_FILESYSTEM, SFGAO_FOLDER},
        },
        UI::{
            HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow},
            Input::KeyboardAndMouse::{EnableWindow, VK_ESCAPE},
            Shell::{
                EBO_SHOWFRAMES, ExplorerBrowser, FOLDERSETTINGS, FVM_THUMBNAIL, FWF_AUTOARRANGE,
                FWF_FULLROWSELECT, FWF_NOWEBVIEW, ICommDlgBrowser, ICommDlgBrowser_Impl,
                ICommDlgBrowser2, ICommDlgBrowser2_Impl, IExplorerBrowser, IExplorerBrowserEvents,
                IExplorerBrowserEvents_Impl, IFolderView2, IShellFolder, IShellItem, IShellView,
                IUnknown_SetSite, SBSP_NAVIGATEBACK, SBSP_PARENT, SHCreateItemFromParsingName,
                SHCreateItemWithParent, SIGDN_FILESYSPATH,
            },
            WindowsAndMessaging::{
                CREATESTRUCTW, CS_DBLCLKS, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
                DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetClientRect, GetMessageW,
                GetWindowLongPtrW, GetWindowRect, HMENU, IDC_ARROW, IsDialogMessageW, LoadCursorW,
                MINMAXINFO, MSG, MoveWindow, NONCLIENTMETRICSW, PostMessageW, PostQuitMessage,
                RegisterClassW, SPI_GETNONCLIENTMETRICS, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SendMessageW, SetForegroundWindow,
                SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, SystemParametersInfoW,
                TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND,
                WM_DESTROY, WM_DPICHANGED, WM_GETMINMAXINFO, WM_KEYDOWN, WM_NCCREATE, WM_SETFONT,
                WM_SIZE, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_EX_CLIENTEDGE,
                WS_EX_CONTROLPARENT, WS_EX_DLGMODALFRAME, WS_MAXIMIZEBOX, WS_OVERLAPPED,
                WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_VISIBLE,
            },
        },
    },
    core::{
        ComObject, Error as WindowsError, HRESULT, Interface, PCWSTR, Ref, Result as WindowsResult,
        implement, w,
    },
};

use crate::selection::VIDEO_EXTENSIONS;

const WINDOW_CLASS: PCWSTR = w!("VidmetryExplorerFolderPicker");
const BUTTON_CLASS: PCWSTR = w!("BUTTON");
const EDIT_CLASS: PCWSTR = w!("EDIT");

const ID_BACK: i32 = 1001;
const ID_UP: i32 = 1002;
const ID_SELECT: i32 = 1003;
const ID_CANCEL: i32 = 1004;

const WM_NAVIGATION_COMPLETE: u32 = WM_APP + 1;

const JAPANESE_PRIMARY_LANGUAGE_ID: u16 = 0x11;
const BASE_DPI: i32 = 96;
const INITIAL_WIDTH: i32 = 980;
const INITIAL_HEIGHT: i32 = 700;
const MINIMUM_WIDTH: i32 = 720;
const MINIMUM_HEIGHT: i32 = 500;
const MARGIN: i32 = 12;
const CONTROL_HEIGHT: i32 = 32;
const SMALL_BUTTON_WIDTH: i32 = 42;
const ACTION_BUTTON_WIDTH: i32 = 112;
const CONTROL_GAP: i32 = 8;

const E_POINTER: HRESULT = HRESULT(0x80004003_u32 as i32);
const E_NOINTERFACE: HRESULT = HRESULT(0x80004002_u32 as i32);
const E_NOTIMPL: HRESULT = HRESULT(0x80004001_u32 as i32);

struct ComApartment;

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

#[implement(IServiceProvider, ICommDlgBrowser2, IExplorerBrowserEvents)]
struct ExplorerSite {
    self_interface: Mutex<Option<windows::core::IUnknown>>,
    window: HWND,
}

impl ExplorerSite {
    fn new(window: HWND) -> ComObject<Self> {
        let site = ComObject::new(Self {
            self_interface: Mutex::new(None),
            window,
        });
        *site
            .get()
            .self_interface
            .lock()
            .expect("explorer site identity") =
            Some(site.to_interface::<windows::core::IUnknown>());
        site
    }

    fn clear_identity(&self) {
        self.self_interface
            .lock()
            .expect("explorer site identity")
            .take();
    }
}

impl IServiceProvider_Impl for ExplorerSite_Impl {
    fn QueryService(
        &self,
        guid_service: *const windows::core::GUID,
        riid: *const windows::core::GUID,
        object: *mut *mut c_void,
    ) -> WindowsResult<()> {
        if guid_service.is_null() || riid.is_null() || object.is_null() {
            return Err(WindowsError::from_hresult(E_POINTER));
        }
        unsafe { *object = null_mut() };
        if unsafe { *guid_service } != ICommDlgBrowser::IID {
            return Err(WindowsError::from_hresult(E_NOINTERFACE));
        }

        let identity = self.self_interface.lock().expect("explorer site identity");
        let identity = identity
            .as_ref()
            .ok_or_else(|| WindowsError::from_hresult(E_NOINTERFACE))?;
        unsafe {
            (Interface::vtable(identity).QueryInterface)(Interface::as_raw(identity), riid, object)
                .ok()
        }
    }
}

impl ICommDlgBrowser_Impl for ExplorerSite_Impl {
    fn OnDefaultCommand(&self, shell_view: Ref<'_, IShellView>) -> WindowsResult<()> {
        let view: IFolderView2 = shell_view
            .as_ref()
            .ok_or_else(|| WindowsError::from_hresult(E_POINTER))?
            .cast()?;
        let selected_index = unsafe { view.GetSelectedItem(-1) }?;
        let item: IShellItem = unsafe { view.GetItem(selected_index) }?;
        if is_folder(&item) {
            // S_FALSE asks the Shell view to perform its normal folder navigation.
            Err(WindowsError::from_hresult(S_FALSE))
        } else {
            // Videos are informative in this picker; do not launch their associated application.
            Ok(())
        }
    }

    fn OnStateChange(&self, _shell_view: Ref<'_, IShellView>, _change: u32) -> WindowsResult<()> {
        Ok(())
    }

    fn IncludeObject(
        &self,
        shell_view: Ref<'_, IShellView>,
        item_id: *const windows::Win32::UI::Shell::Common::ITEMIDLIST,
    ) -> WindowsResult<()> {
        let view: IFolderView2 = shell_view
            .as_ref()
            .ok_or_else(|| WindowsError::from_hresult(E_POINTER))?
            .cast()?;
        let folder: IShellFolder = unsafe { view.GetFolder() }?;
        let item: IShellItem = unsafe { SHCreateItemWithParent(None, &folder, item_id) }?;
        if should_show(&item) {
            Ok(())
        } else {
            Err(WindowsError::from_hresult(S_FALSE))
        }
    }
}

impl ICommDlgBrowser2_Impl for ExplorerSite_Impl {
    fn Notify(&self, _shell_view: Ref<'_, IShellView>, _notification: u32) -> WindowsResult<()> {
        Ok(())
    }

    fn GetDefaultMenuText(
        &self,
        _shell_view: Ref<'_, IShellView>,
        _text: windows::core::PWSTR,
        _maximum_length: i32,
    ) -> WindowsResult<()> {
        Err(WindowsError::from_hresult(E_NOTIMPL))
    }

    fn GetViewFlags(&self) -> WindowsResult<u32> {
        // IncludeObject must be called so unsupported files can be removed from each view.
        Ok(0)
    }
}

impl IExplorerBrowserEvents_Impl for ExplorerSite_Impl {
    fn OnNavigationPending(
        &self,
        _folder: *const windows::Win32::UI::Shell::Common::ITEMIDLIST,
    ) -> WindowsResult<()> {
        Ok(())
    }

    fn OnViewCreated(&self, _shell_view: Ref<'_, IShellView>) -> WindowsResult<()> {
        Ok(())
    }

    fn OnNavigationComplete(
        &self,
        _folder: *const windows::Win32::UI::Shell::Common::ITEMIDLIST,
    ) -> WindowsResult<()> {
        unsafe {
            PostMessageW(
                Some(self.window),
                WM_NAVIGATION_COMPLETE,
                WPARAM(0),
                LPARAM(0),
            )?
        };
        Ok(())
    }

    fn OnNavigationFailed(
        &self,
        _folder: *const windows::Win32::UI::Shell::Common::ITEMIDLIST,
    ) -> WindowsResult<()> {
        Ok(())
    }
}

struct PickerWindow {
    window: HWND,
    back_button: HWND,
    up_button: HWND,
    path_box: HWND,
    select_button: HWND,
    cancel_button: HWND,
    browser: Option<IExplorerBrowser>,
    browser_cookie: Option<u32>,
    site: Option<ComObject<ExplorerSite>>,
    font: Option<HFONT>,
    selected_path: Option<String>,
    select_folder_label: Vec<u16>,
    cancel_label: Vec<u16>,
}

impl PickerWindow {
    fn new(select_folder_label: &str, cancel_label: &str) -> Self {
        Self {
            window: HWND::default(),
            back_button: HWND::default(),
            up_button: HWND::default(),
            path_box: HWND::default(),
            select_button: HWND::default(),
            cancel_button: HWND::default(),
            browser: None,
            browser_cookie: None,
            site: None,
            font: None,
            selected_path: None,
            select_folder_label: wide_string(select_folder_label),
            cancel_label: wide_string(cancel_label),
        }
    }

    fn initialize(&mut self, initial_directory: Option<&str>) -> WindowsResult<()> {
        self.create_controls()?;

        let browser: IExplorerBrowser =
            unsafe { CoCreateInstance(&ExplorerBrowser, None, CLSCTX_INPROC_SERVER)? };
        let site = ExplorerSite::new(self.window);
        let service_provider: IServiceProvider = site.to_interface();
        unsafe { IUnknown_SetSite(&browser, &service_provider)? };

        let settings = FOLDERSETTINGS {
            ViewMode: FVM_THUMBNAIL.0 as u32,
            fFlags: (FWF_AUTOARRANGE | FWF_FULLROWSELECT | FWF_NOWEBVIEW).0 as u32,
        };
        let browser_rect = self.browser_rect();
        unsafe {
            browser.Initialize(self.window, &browser_rect, Some(&settings))?;
            browser.SetOptions(EBO_SHOWFRAMES)?;
        }
        let events: IExplorerBrowserEvents = site.to_interface();
        let cookie = unsafe { browser.Advise(&events) }?;

        self.browser = Some(browser);
        self.browser_cookie = Some(cookie);
        self.site = Some(site);

        let initial_item = initial_directory
            .and_then(|path| shell_item_from_path(path).ok())
            .or_else(|| shell_item_from_path(&default_initial_directory()).ok())
            .ok_or_else(|| WindowsError::from_hresult(HRESULT(0x80070003_u32 as i32)))?;
        unsafe {
            self.browser
                .as_ref()
                .expect("explorer browser")
                .BrowseToObject(&initial_item, 0)?;
        }
        Ok(())
    }

    fn create_controls(&mut self) -> WindowsResult<()> {
        let instance = module_instance()?;
        self.back_button = create_control(
            BUTTON_CLASS,
            w!("\u{2190}"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            WINDOW_EX_STYLE(0),
            self.window,
            ID_BACK,
            instance,
        )?;
        self.up_button = create_control(
            BUTTON_CLASS,
            w!("\u{2191}"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            WINDOW_EX_STYLE(0),
            self.window,
            ID_UP,
            instance,
        )?;
        self.path_box = create_control(
            EDIT_CLASS,
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(0x0800),
            WS_EX_CLIENTEDGE,
            self.window,
            0,
            instance,
        )?;
        self.select_button = create_control(
            BUTTON_CLASS,
            PCWSTR(self.select_folder_label.as_ptr()),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(0x0001),
            WINDOW_EX_STYLE(0),
            self.window,
            ID_SELECT,
            instance,
        )?;
        self.cancel_button = create_control(
            BUTTON_CLASS,
            PCWSTR(self.cancel_label.as_ptr()),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            WINDOW_EX_STYLE(0),
            self.window,
            ID_CANCEL,
            instance,
        )?;

        self.font = create_message_font();
        if let Some(font) = self.font {
            for control in [
                self.back_button,
                self.up_button,
                self.path_box,
                self.select_button,
                self.cancel_button,
            ] {
                unsafe {
                    SendMessageW(
                        control,
                        WM_SETFONT,
                        Some(WPARAM(font.0 as usize)),
                        Some(LPARAM(1)),
                    );
                }
            }
        }
        self.layout();
        Ok(())
    }

    fn layout(&self) {
        if self.window.0.is_null() {
            return;
        }
        let dpi = unsafe { GetDpiForWindow(self.window) } as i32;
        let scale = |value| value * dpi / BASE_DPI;
        let mut client = RECT::default();
        if unsafe { GetClientRect(self.window, &mut client) }.is_err() {
            return;
        }

        let margin = scale(MARGIN);
        let gap = scale(CONTROL_GAP);
        let control_height = scale(CONTROL_HEIGHT);
        let small_width = scale(SMALL_BUTTON_WIDTH);
        let action_width = scale(ACTION_BUTTON_WIDTH);
        let width = client.right - client.left;
        let height = client.bottom - client.top;
        let top = margin;
        let bottom = height - margin - control_height;

        let _ = unsafe {
            MoveWindow(
                self.back_button,
                margin,
                top,
                small_width,
                control_height,
                true,
            )
        };
        let _ = unsafe {
            MoveWindow(
                self.up_button,
                margin + small_width + gap,
                top,
                small_width,
                control_height,
                true,
            )
        };
        let path_left = margin + (small_width + gap) * 2;
        let _ = unsafe {
            MoveWindow(
                self.path_box,
                path_left,
                top,
                (width - margin - path_left).max(1),
                control_height,
                true,
            )
        };

        let _ = unsafe {
            MoveWindow(
                self.cancel_button,
                width - margin - action_width,
                bottom,
                action_width,
                control_height,
                true,
            )
        };
        let _ = unsafe {
            MoveWindow(
                self.select_button,
                width - margin - action_width * 2 - gap,
                bottom,
                action_width,
                control_height,
                true,
            )
        };

        if let Some(browser) = &self.browser {
            let _ = unsafe { browser.SetRect(None, self.browser_rect()) };
        }
    }

    fn browser_rect(&self) -> RECT {
        let dpi = unsafe { GetDpiForWindow(self.window) } as i32;
        let scale = |value| value * dpi / BASE_DPI;
        let mut client = RECT::default();
        let _ = unsafe { GetClientRect(self.window, &mut client) };
        RECT {
            left: scale(MARGIN),
            top: scale(MARGIN + CONTROL_HEIGHT + CONTROL_GAP),
            right: client.right - scale(MARGIN),
            bottom: client.bottom - scale(MARGIN + CONTROL_HEIGHT + CONTROL_GAP),
        }
    }

    fn navigate(&self, flags: u32) {
        if let Some(browser) = &self.browser {
            let _ = unsafe { browser.BrowseToIDList(null(), flags) };
        }
    }

    fn update_current_folder(&self) {
        let path = self.current_folder_path();
        let label = wide_string(path.as_deref().unwrap_or(""));
        let _ = unsafe { SetWindowTextW(self.path_box, PCWSTR(label.as_ptr())) };
        let _ = unsafe { EnableWindow(self.select_button, path.is_some()) };
    }

    fn current_folder_path(&self) -> Option<String> {
        let browser = self.browser.as_ref()?;
        let view: IFolderView2 = unsafe { browser.GetCurrentView() }.ok()?;
        let item: IShellItem = unsafe { view.GetFolder() }.ok()?;
        shell_item_path(&item).ok()
    }

    fn accept(&mut self) {
        if let Some(path) = self.current_folder_path() {
            self.selected_path = Some(path);
            let _ = unsafe { DestroyWindow(self.window) };
        }
    }

    fn destroy_browser(&mut self) {
        if let Some(browser) = self.browser.take() {
            let _ = unsafe { IUnknown_SetSite(&browser, None) };
            if let Some(cookie) = self.browser_cookie.take() {
                let _ = unsafe { browser.Unadvise(cookie) };
            }
            let _ = unsafe { browser.Destroy() };
        }
        if let Some(site) = self.site.take() {
            site.clear_identity();
        }
        if let Some(font) = self.font.take() {
            let _ = unsafe { DeleteObject(font.into()) };
        }
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        let state = create.lpCreateParams as *mut PickerWindow;
        unsafe { SetWindowLongPtrW(window, GWLP_USERDATA, state as isize) };
        if !state.is_null() {
            unsafe { (*state).window = window };
        }
    }

    let state = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) as *mut PickerWindow };
    if state.is_null() {
        return unsafe { DefWindowProcW(window, message, wparam, lparam) };
    }
    let state = unsafe { &mut *state };

    match message {
        WM_SIZE => {
            state.layout();
            LRESULT(0)
        }
        WM_COMMAND => {
            match (wparam.0 & 0xffff) as i32 {
                ID_BACK => state.navigate(SBSP_NAVIGATEBACK),
                ID_UP => state.navigate(SBSP_PARENT),
                ID_SELECT => state.accept(),
                ID_CANCEL => {
                    let _ = unsafe { DestroyWindow(window) };
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_NAVIGATION_COMPLETE => {
            state.update_current_folder();
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            let info = unsafe { &mut *(lparam.0 as *mut MINMAXINFO) };
            let dpi = unsafe { GetDpiForWindow(window) } as i32;
            info.ptMinTrackSize.x = MINIMUM_WIDTH * dpi / BASE_DPI;
            info.ptMinTrackSize.y = MINIMUM_HEIGHT * dpi / BASE_DPI;
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let suggested = unsafe { &*(lparam.0 as *const RECT) };
            let _ = unsafe {
                SetWindowPos(
                    window,
                    None,
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                )
            };
            state.layout();
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = unsafe { DestroyWindow(window) };
            LRESULT(0)
        }
        WM_DESTROY => {
            state.destroy_browser();
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

pub(super) fn pick(
    owner: isize,
    title: &str,
    select_folder_label: &str,
    cancel_label: &str,
    initial_directory: Option<&str>,
) -> Result<Option<String>, String> {
    let initialization =
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    initialization.ok().map_err(windows_error)?;
    let _com_apartment = ComApartment;

    register_window_class().map_err(windows_error)?;
    let owner = HWND(owner as *mut c_void);
    let mut picker = Box::new(PickerWindow::new(select_folder_label, cancel_label));
    let window = create_picker_window(owner, title, picker.as_mut()).map_err(windows_error)?;

    if let Err(error) = picker.initialize(initial_directory) {
        let _ = unsafe { DestroyWindow(window) };
        return Err(windows_error(error));
    }

    let _ = unsafe { EnableWindow(owner, false) };
    unsafe {
        let _ = ShowWindow(window, SW_SHOW);
        let _ = UpdateWindow(window);
    }

    let mut message = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) }.0;
        if result <= 0 {
            break;
        }
        if message.message == WM_KEYDOWN && message.wParam.0 == VK_ESCAPE.0 as usize {
            let _ = unsafe { PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0)) };
            continue;
        }
        if !unsafe { IsDialogMessageW(window, &message) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    let _ = unsafe { EnableWindow(owner, true) };
    let _ = unsafe { SetForegroundWindow(owner) };
    Ok(picker.selected_path.clone())
}

fn register_window_class() -> WindowsResult<()> {
    let instance = module_instance()?;
    let class = WNDCLASSW {
        style: CS_DBLCLKS,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
        hbrBackground: HBRUSH((COLOR_WINDOW.0 as usize + 1) as *mut c_void),
        lpszClassName: WINDOW_CLASS,
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        let error = unsafe { GetLastError() };
        if error != ERROR_CLASS_ALREADY_EXISTS {
            return Err(WindowsError::from_win32());
        }
    }
    Ok(())
}

fn create_picker_window(
    owner: HWND,
    title: &str,
    picker: &mut PickerWindow,
) -> WindowsResult<HWND> {
    let dpi = unsafe { GetDpiForWindow(owner) }.max(BASE_DPI as u32) as i32;
    let style =
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MAXIMIZEBOX | WS_CLIPCHILDREN;
    let ex_style = WS_EX_DLGMODALFRAME | WS_EX_CONTROLPARENT;
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: INITIAL_WIDTH * dpi / BASE_DPI,
        bottom: INITIAL_HEIGHT * dpi / BASE_DPI,
    };
    unsafe { AdjustWindowRectExForDpi(&mut rect, style, false, ex_style, dpi as u32)? };
    let (x, y) = centered_position(owner, rect.right - rect.left, rect.bottom - rect.top);
    let title = wide_string(title);
    unsafe {
        CreateWindowExW(
            ex_style,
            WINDOW_CLASS,
            PCWSTR(title.as_ptr()),
            style,
            x,
            y,
            rect.right - rect.left,
            rect.bottom - rect.top,
            Some(owner),
            None,
            Some(module_instance()?),
            Some(picker as *mut PickerWindow as *const c_void),
        )
    }
}

fn create_control<P1, P2>(
    class: P1,
    text: P2,
    style: WINDOW_STYLE,
    ex_style: WINDOW_EX_STYLE,
    parent: HWND,
    id: i32,
    instance: HINSTANCE,
) -> WindowsResult<HWND>
where
    P1: windows::core::Param<PCWSTR>,
    P2: windows::core::Param<PCWSTR>,
{
    unsafe {
        CreateWindowExW(
            ex_style,
            class,
            text,
            style,
            0,
            0,
            0,
            0,
            Some(parent),
            Some(HMENU(id as *mut c_void)),
            Some(instance),
            None,
        )
    }
}

fn centered_position(owner: HWND, width: i32, height: i32) -> (i32, i32) {
    let mut owner_rect = RECT::default();
    if unsafe { GetWindowRect(owner, &mut owner_rect) }.is_err() {
        return (CW_USEDEFAULT, CW_USEDEFAULT);
    }
    let monitor = unsafe { MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let work = if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        info.rcWork
    } else {
        owner_rect
    };
    let x = (owner_rect.left + (owner_rect.right - owner_rect.left - width) / 2)
        .clamp(work.left, (work.right - width).max(work.left));
    let y = (owner_rect.top + (owner_rect.bottom - owner_rect.top - height) / 2)
        .clamp(work.top, (work.bottom - height).max(work.top));
    (x, y)
}

fn create_message_font() -> Option<HFONT> {
    let mut metrics = NONCLIENTMETRICSW {
        cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
        ..Default::default()
    };
    unsafe {
        SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            metrics.cbSize,
            Some((&mut metrics as *mut _) as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .ok()?;
    }
    let font = unsafe { CreateFontIndirectW(&metrics.lfMessageFont) };
    (!font.is_invalid()).then_some(font)
}

fn module_instance() -> WindowsResult<HINSTANCE> {
    let module = unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None)? };
    Ok(HINSTANCE(module.0))
}

fn shell_item_from_path(path: &str) -> WindowsResult<IShellItem> {
    let path = wide_string(path);
    unsafe { SHCreateItemFromParsingName(PCWSTR(path.as_ptr()), None) }
}

fn shell_item_path(item: &IShellItem) -> WindowsResult<String> {
    let path = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH)? };
    let result = unsafe { path.to_string() };
    unsafe { CoTaskMemFree(Some(path.0.cast())) };
    Ok(result?)
}

fn is_folder(item: &IShellItem) -> bool {
    unsafe { item.GetAttributes(SFGAO_FOLDER) }
        .map(|attributes| attributes.contains(SFGAO_FOLDER))
        .unwrap_or(false)
}

fn should_show(item: &IShellItem) -> bool {
    let attributes = match unsafe { item.GetAttributes(SFGAO_FOLDER | SFGAO_FILESYSTEM) } {
        Ok(attributes) => attributes,
        Err(_) => return false,
    };
    if attributes.contains(SFGAO_FOLDER) {
        return true;
    }
    if !attributes.contains(SFGAO_FILESYSTEM) {
        return false;
    }
    shell_item_path(item)
        .ok()
        .as_deref()
        .is_some_and(is_supported_video_path)
}

fn is_supported_video_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            VIDEO_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

fn default_initial_directory() -> String {
    std::env::var("USERPROFILE").unwrap_or_else(|_| String::from("C:\\"))
}

fn ui_language(language_id: u16) -> &'static str {
    if language_id & 0x03ff == JAPANESE_PRIMARY_LANGUAGE_ID {
        "ja"
    } else {
        "en"
    }
}

pub(super) fn windows_ui_language() -> &'static str {
    ui_language(unsafe { GetUserDefaultUILanguage() })
}

fn wide_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn windows_error(error: windows::core::Error) -> String {
    if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
        return String::from("cancelled");
    }
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_video_filter_accepts_every_supported_extension_case_insensitively() {
        for extension in VIDEO_EXTENSIONS {
            assert!(is_supported_video_path(&format!("C:\\video.{extension}")));
            assert!(is_supported_video_path(&format!(
                "C:\\video.{}",
                extension.to_uppercase()
            )));
        }
        assert!(!is_supported_video_path("C:\\video.txt"));
        assert!(!is_supported_video_path("C:\\video"));
    }

    #[test]
    fn folder_dialog_uses_windows_ui_language() {
        assert_eq!(ui_language(0x0411), "ja");
        assert_eq!(ui_language(0x0409), "en");
    }
}
