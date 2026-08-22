use std::{
    ffi::c_void,
    path::Path,
    ptr::{null, null_mut},
    sync::Mutex,
};

use windows::{
    Foundation::TypedEventHandler,
    UI::{
        Color,
        ViewManagement::{UIColorType, UISettings},
    },
    Win32::{
        Foundation::{
            COLORREF, ERROR_CANCELLED, ERROR_CLASS_ALREADY_EXISTS, GetLastError, HINSTANCE, HWND,
            LPARAM, LRESULT, RECT, S_FALSE, WPARAM,
        },
        Globalization::GetUserDefaultUILanguage,
        Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute},
        Graphics::Gdi::{
            BeginPaint, CreateFontIndirectW, CreateSolidBrush, DT_CENTER, DT_END_ELLIPSIS,
            DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawFocusRect, DrawTextW,
            EndPaint, FillRect, FrameRect, GetMonitorInfoW, HBRUSH, HDC, HFONT, InvalidateRect,
            MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, OPAQUE, PAINTSTRUCT,
            RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow, SelectObject, SetBkColor,
            SetBkMode, SetTextColor, TRANSPARENT, UpdateWindow,
        },
        System::{
            Com::{
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IServiceProvider,
                IServiceProvider_Impl,
            },
            SystemServices::{SFGAO_FILESYSTEM, SFGAO_FOLDER},
            WinRT::{RO_INIT_SINGLETHREADED, RoInitialize, RoUninitialize},
        },
        UI::{
            Controls::{DRAWITEMSTRUCT, ODS_DISABLED, ODS_FOCUS, ODS_HOTLIGHT, ODS_SELECTED},
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
                BS_OWNERDRAW, CREATESTRUCTW, CS_DBLCLKS, CW_USEDEFAULT, CreateWindowExW,
                DefWindowProcW, DestroyWindow, DispatchMessageW, ES_READONLY, GWLP_USERDATA,
                GetClientRect, GetMessageW, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW,
                GetWindowTextW, HMENU, IDC_ARROW, IsDialogMessageW, LoadCursorW, MINMAXINFO, MSG,
                MoveWindow, NONCLIENTMETRICSW, PostMessageW, PostQuitMessage, RegisterClassW,
                SPI_GETNONCLIENTMETRICS, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SendMessageW, SetForegroundWindow,
                SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, SystemParametersInfoW,
                TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND,
                WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DPICHANGED, WM_DRAWITEM,
                WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN, WM_NCCREATE, WM_PAINT, WM_SETFONT,
                WM_SETTINGCHANGE, WM_SIZE, WM_THEMECHANGED, WNDCLASSW, WS_CAPTION, WS_CHILD,
                WS_CLIPCHILDREN, WS_EX_CONTROLPARENT, WS_EX_DLGMODALFRAME, WS_MAXIMIZEBOX,
                WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_VISIBLE,
            },
        },
    },
    core::{
        ComObject, Error as WindowsError, HRESULT, IInspectable, Interface, PCWSTR, Ref,
        Result as WindowsResult, implement, w,
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
const WM_COLOR_MODE_CHANGED: u32 = WM_APP + 2;

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

#[derive(Clone, Copy)]
struct ThemePalette {
    dark: bool,
    background: COLORREF,
    surface: COLORREF,
    hover: COLORREF,
    pressed: COLORREF,
    border: COLORREF,
    text: COLORREF,
    disabled_text: COLORREF,
    accent: COLORREF,
}

impl ThemePalette {
    fn fallback_light() -> Self {
        Self::from_colors(
            Color {
                A: 255,
                R: 0,
                G: 0,
                B: 0,
            },
            Color {
                A: 255,
                R: 255,
                G: 255,
                B: 255,
            },
            Color {
                A: 255,
                R: 0,
                G: 120,
                B: 212,
            },
        )
    }

    fn from_settings(settings: &UISettings) -> WindowsResult<Self> {
        Ok(Self::from_colors(
            settings.GetColorValue(UIColorType::Foreground)?,
            settings.GetColorValue(UIColorType::Background)?,
            settings.GetColorValue(UIColorType::Accent)?,
        ))
    }

    fn from_colors(foreground: Color, background: Color, accent: Color) -> Self {
        let dark = is_color_light(foreground);
        let background_weight = if dark { 10 } else { 5 };
        let surface_weight = if dark { 15 } else { 0 };
        let hover_weight = if dark { 20 } else { 5 };
        let pressed_weight = if dark { 25 } else { 10 };
        Self {
            dark,
            background: colorref(blend(background, foreground, background_weight)),
            surface: colorref(blend(background, foreground, surface_weight)),
            hover: colorref(blend(background, foreground, hover_weight)),
            pressed: colorref(blend(background, foreground, pressed_weight)),
            border: colorref(blend(background, foreground, 35)),
            text: colorref(foreground),
            disabled_text: colorref(blend(background, foreground, 50)),
            accent: colorref(accent),
        }
    }
}

struct ThemeBrushes {
    background: HBRUSH,
    surface: HBRUSH,
    hover: HBRUSH,
    pressed: HBRUSH,
    border: HBRUSH,
    accent: HBRUSH,
}

impl ThemeBrushes {
    fn new(palette: ThemePalette) -> Self {
        unsafe {
            Self {
                background: CreateSolidBrush(palette.background),
                surface: CreateSolidBrush(palette.surface),
                hover: CreateSolidBrush(palette.hover),
                pressed: CreateSolidBrush(palette.pressed),
                border: CreateSolidBrush(palette.border),
                accent: CreateSolidBrush(palette.accent),
            }
        }
    }
}

impl Drop for ThemeBrushes {
    fn drop(&mut self) {
        for brush in [
            self.background,
            self.surface,
            self.hover,
            self.pressed,
            self.border,
            self.accent,
        ] {
            let _ = unsafe { DeleteObject(brush.into()) };
        }
    }
}

fn is_color_light(color: Color) -> bool {
    (5 * u32::from(color.G)) + (2 * u32::from(color.R)) + u32::from(color.B) > 8 * 128
}

fn blend(background: Color, foreground: Color, foreground_percent: u16) -> Color {
    let mix = |background: u8, foreground: u8| {
        (((u16::from(background) * (100 - foreground_percent))
            + (u16::from(foreground) * foreground_percent))
            / 100) as u8
    };
    Color {
        A: 255,
        R: mix(background.R, foreground.R),
        G: mix(background.G, foreground.G),
        B: mix(background.B, foreground.B),
    }
}

fn colorref(color: Color) -> COLORREF {
    COLORREF(u32::from(color.R) | (u32::from(color.G) << 8) | (u32::from(color.B) << 16))
}

struct ComApartment;

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

struct WinRtApartment;

impl Drop for WinRtApartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
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
    ui_settings: Option<UISettings>,
    color_values_changed_token: Option<i64>,
    palette: ThemePalette,
    brushes: ThemeBrushes,
    selected_path: Option<String>,
    select_folder_label: Vec<u16>,
    cancel_label: Vec<u16>,
}

impl PickerWindow {
    fn new(select_folder_label: &str, cancel_label: &str) -> Self {
        let palette = ThemePalette::fallback_light();
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
            ui_settings: None,
            color_values_changed_token: None,
            palette,
            brushes: ThemeBrushes::new(palette),
            selected_path: None,
            select_folder_label: wide_string(select_folder_label),
            cancel_label: wide_string(cancel_label),
        }
    }

    fn initialize(&mut self, initial_directory: Option<&str>) -> WindowsResult<()> {
        self.initialize_color_mode_tracking();
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

    fn initialize_color_mode_tracking(&mut self) {
        let Ok(settings) = UISettings::new() else {
            self.apply_color_mode(ThemePalette::fallback_light());
            return;
        };
        let window = self.window.0 as isize;
        let handler = TypedEventHandler::<UISettings, IInspectable>::new(move |_, _| {
            let window = HWND(window as *mut c_void);
            let _ =
                unsafe { PostMessageW(Some(window), WM_COLOR_MODE_CHANGED, WPARAM(0), LPARAM(0)) };
            Ok(())
        });
        self.color_values_changed_token = settings.ColorValuesChanged(&handler).ok();
        self.ui_settings = Some(settings);
        self.refresh_color_mode();
    }

    fn refresh_color_mode(&mut self) {
        let palette = self
            .ui_settings
            .as_ref()
            .and_then(|settings| ThemePalette::from_settings(settings).ok())
            .unwrap_or_else(ThemePalette::fallback_light);
        self.apply_color_mode(palette);
    }

    fn apply_color_mode(&mut self, palette: ThemePalette) {
        self.palette = palette;
        self.brushes = ThemeBrushes::new(palette);

        let dark_mode = windows::core::BOOL::from(palette.dark);
        let _ = unsafe {
            DwmSetWindowAttribute(
                self.window,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&dark_mode as *const windows::core::BOOL).cast(),
                std::mem::size_of_val(&dark_mode) as u32,
            )
        };
        let _ = unsafe {
            RedrawWindow(
                Some(self.window),
                None,
                None,
                RDW_FRAME | RDW_INVALIDATE | RDW_UPDATENOW,
            )
        };
        for control in [
            self.back_button,
            self.up_button,
            self.path_box,
            self.select_button,
            self.cancel_button,
        ] {
            if !control.0.is_null() {
                let _ = unsafe { InvalidateRect(Some(control), None, true) };
            }
        }
    }

    fn paint(&self) {
        let mut paint = PAINTSTRUCT::default();
        let dc = unsafe { BeginPaint(self.window, &mut paint) };
        self.paint_background(dc);
        let _ = unsafe { EndPaint(self.window, &paint) };
    }

    fn paint_background(&self, dc: HDC) {
        let mut client = RECT::default();
        if unsafe { GetClientRect(self.window, &mut client) }.is_err() {
            return;
        }
        unsafe {
            FillRect(dc, &client, self.brushes.background);
            if !self.path_box.0.is_null() {
                FrameRect(dc, &self.path_frame_rect(), self.brushes.border);
            }
        }
    }

    fn path_frame_rect(&self) -> RECT {
        let dpi = unsafe { GetDpiForWindow(self.window) } as i32;
        let scale = |value| value * dpi / BASE_DPI;
        let margin = scale(MARGIN);
        let gap = scale(CONTROL_GAP);
        let small_width = scale(SMALL_BUTTON_WIDTH);
        let path_left = margin + (small_width + gap) * 2;
        let mut client = RECT::default();
        let _ = unsafe { GetClientRect(self.window, &mut client) };
        RECT {
            left: path_left,
            top: margin,
            right: client.right - margin,
            bottom: margin + scale(CONTROL_HEIGHT),
        }
    }

    fn draw_button(&self, draw: &DRAWITEMSTRUCT) -> bool {
        if ![
            self.back_button,
            self.up_button,
            self.select_button,
            self.cancel_button,
        ]
        .contains(&draw.hwndItem)
        {
            return false;
        }

        let has_state = |state: u32| draw.itemState.0 & state != 0;
        let brush = if has_state(ODS_SELECTED.0) {
            self.brushes.pressed
        } else if has_state(ODS_HOTLIGHT.0) {
            self.brushes.hover
        } else {
            self.brushes.surface
        };
        let focused = has_state(ODS_FOCUS.0);
        let border = if focused || draw.hwndItem == self.select_button {
            self.brushes.accent
        } else {
            self.brushes.border
        };

        unsafe {
            FillRect(draw.hDC, &draw.rcItem, brush);
            FrameRect(draw.hDC, &draw.rcItem, border);
            SetBkMode(draw.hDC, TRANSPARENT);
            SetTextColor(
                draw.hDC,
                if has_state(ODS_DISABLED.0) {
                    self.palette.disabled_text
                } else {
                    self.palette.text
                },
            );
        }
        let previous_font = self
            .font
            .map(|font| unsafe { SelectObject(draw.hDC, font.into()) });

        let length = unsafe { GetWindowTextLengthW(draw.hwndItem) }.max(0) as usize;
        let mut text = vec![0_u16; length + 1];
        let copied = unsafe { GetWindowTextW(draw.hwndItem, &mut text) }.max(0) as usize;
        text.truncate(copied);
        let mut text_rect = draw.rcItem;
        if has_state(ODS_SELECTED.0) {
            text_rect.left += 1;
            text_rect.top += 1;
        }
        unsafe {
            DrawTextW(
                draw.hDC,
                &mut text,
                &mut text_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
            );
        }
        if let Some(previous_font) = previous_font {
            let _ = unsafe { SelectObject(draw.hDC, previous_font) };
        }
        if focused {
            let mut focus = draw.rcItem;
            let inset = (unsafe { GetDpiForWindow(self.window) } as i32 / BASE_DPI).max(1) * 3;
            focus.left += inset;
            focus.top += inset;
            focus.right -= inset;
            focus.bottom -= inset;
            let _ = unsafe { DrawFocusRect(draw.hDC, &focus) };
        }
        true
    }

    fn path_box_color(&self, dc: HDC) -> LRESULT {
        unsafe {
            SetBkMode(dc, OPAQUE);
            SetBkColor(dc, self.palette.surface);
            SetTextColor(dc, self.palette.text);
        }
        LRESULT(self.brushes.surface.0 as isize)
    }

    fn create_controls(&mut self) -> WindowsResult<()> {
        let instance = module_instance()?;
        self.back_button = create_control(
            BUTTON_CLASS,
            w!("\u{2190}"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            WINDOW_EX_STYLE(0),
            self.window,
            ID_BACK,
            instance,
        )?;
        self.up_button = create_control(
            BUTTON_CLASS,
            w!("\u{2191}"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            WINDOW_EX_STYLE(0),
            self.window,
            ID_UP,
            instance,
        )?;
        self.path_box = create_control(
            EDIT_CLASS,
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_READONLY as u32),
            WINDOW_EX_STYLE(0),
            self.window,
            0,
            instance,
        )?;
        self.select_button = create_control(
            BUTTON_CLASS,
            PCWSTR(self.select_folder_label.as_ptr()),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            WINDOW_EX_STYLE(0),
            self.window,
            ID_SELECT,
            instance,
        )?;
        self.cancel_button = create_control(
            BUTTON_CLASS,
            PCWSTR(self.cancel_label.as_ptr()),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
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
        let path_width = (width - margin - path_left).max(1);
        let path_border = scale(1).max(1);
        let _ = unsafe {
            MoveWindow(
                self.path_box,
                path_left + path_border,
                top + path_border,
                (path_width - path_border * 2).max(1),
                (control_height - path_border * 2).max(1),
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
        if let Some(settings) = self.ui_settings.take()
            && let Some(token) = self.color_values_changed_token.take()
        {
            let _ = settings.RemoveColorValuesChanged(token);
        }
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
        WM_PAINT => {
            state.paint();
            LRESULT(0)
        }
        WM_ERASEBKGND => {
            state.paint_background(HDC(wparam.0 as *mut c_void));
            LRESULT(1)
        }
        WM_DRAWITEM => {
            let draw = unsafe { (lparam.0 as *const DRAWITEMSTRUCT).as_ref() };
            if draw.is_some_and(|draw| state.draw_button(draw)) {
                LRESULT(1)
            } else {
                unsafe { DefWindowProcW(window, message, wparam, lparam) }
            }
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            if HWND(lparam.0 as *mut c_void) == state.path_box {
                state.path_box_color(HDC(wparam.0 as *mut c_void))
            } else {
                unsafe { DefWindowProcW(window, message, wparam, lparam) }
            }
        }
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
        WM_COLOR_MODE_CHANGED | WM_SETTINGCHANGE | WM_THEMECHANGED => {
            state.refresh_color_mode();
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
    unsafe { RoInitialize(RO_INIT_SINGLETHREADED) }.map_err(windows_error)?;
    let _winrt_apartment = WinRtApartment;

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
        hbrBackground: HBRUSH::default(),
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

    #[test]
    fn color_mode_uses_the_windows_foreground_brightness_rule() {
        assert!(is_color_light(Color {
            A: 255,
            R: 255,
            G: 255,
            B: 255,
        }));
        assert!(!is_color_light(Color {
            A: 255,
            R: 0,
            G: 0,
            B: 0,
        }));

        let dark = ThemePalette::from_colors(
            Color {
                A: 255,
                R: 255,
                G: 255,
                B: 255,
            },
            Color {
                A: 255,
                R: 0,
                G: 0,
                B: 0,
            },
            Color {
                A: 255,
                R: 0,
                G: 120,
                B: 212,
            },
        );
        let light = ThemePalette::fallback_light();
        assert!(dark.dark);
        assert!(!light.dark);
        assert_eq!(dark.background, COLORREF(0x0019_1919));
        assert_eq!(light.background, COLORREF(0x00f2_f2f2));
    }
}
