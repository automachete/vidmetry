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
            LPARAM, LRESULT, POINT, RECT, S_FALSE, WPARAM,
        },
        Globalization::GetUserDefaultUILanguage,
        Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute},
        Graphics::Gdi::{
            BeginPaint, CreateFontIndirectW, CreatePen, CreateSolidBrush, DT_CALCRECT, DT_CENTER,
            DT_END_ELLIPSIS, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawFocusRect,
            DrawTextW, EndPaint, FillRect, FrameRect, GetDC, GetMonitorInfoW, HBRUSH, HDC, HFONT,
            InvalidateRect, LineTo, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
            MoveToEx, OPAQUE, PAINTSTRUCT, PS_SOLID, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW,
            RedrawWindow, ReleaseDC, ScreenToClient, SelectObject, SetBkColor, SetBkMode,
            SetTextColor, TRANSPARENT, UpdateWindow,
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
            Controls::{
                DRAWITEMSTRUCT, ODS_DISABLED, ODS_FOCUS, ODS_HOTLIGHT, ODS_SELECTED,
                SetWindowTheme, TOOLTIPS_CLASS, TTF_IDISHWND, TTF_SUBCLASS, TTM_ADDTOOLW,
                TTS_ALWAYSTIP, TTS_NOPREFIX, TTTOOLINFOW,
            },
            HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow},
            Input::KeyboardAndMouse::{EnableWindow, VK_ESCAPE},
            Shell::{
                EBO_NOBORDER, EBO_SHOWFRAMES, EP_AdvQueryPane, EP_Commands, EP_Commands_Organize,
                EP_Commands_View, EP_DetailsPane, EP_NavPane, EP_PreviewPane, EP_QueryPane,
                EP_Ribbon, EP_StatusBar, EPS_DEFAULT_OFF, EPS_DEFAULT_ON, EPS_FORCE,
                ExplorerBrowser, FOLDERSETTINGS, FOLDERVIEWMODE, FVM_CONTENT, FVM_DETAILS,
                FVM_ICON, FVM_LIST, FVM_SMALLICON, FVM_THUMBNAIL, FVM_TILE, FWF_AUTOARRANGE,
                FWF_FULLROWSELECT, FWF_NOWEBVIEW, ICommDlgBrowser, ICommDlgBrowser_Impl,
                ICommDlgBrowser2, ICommDlgBrowser2_Impl, IExplorerBrowser, IExplorerBrowserEvents,
                IExplorerBrowserEvents_Impl, IExplorerPaneVisibility, IExplorerPaneVisibility_Impl,
                IFolderView2, IShellFolder, IShellItem, IShellView, IUnknown_SetSite,
                SBSP_NAVIGATEBACK, SBSP_NAVIGATEFORWARD, SBSP_PARENT, SHCreateItemFromParsingName,
                SHCreateItemWithParent, SIGDN_FILESYSPATH,
            },
            WindowsAndMessaging::{
                AppendMenuW, BS_OWNERDRAW, CREATESTRUCTW, CS_DBLCLKS, CW_USEDEFAULT,
                CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
                DispatchMessageW, GWLP_USERDATA, GetClientRect, GetCursorPos, GetMessageW,
                GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, HMENU,
                IDC_ARROW, IsDialogMessageW, LoadCursorW, MF_CHECKED, MF_STRING, MINMAXINFO, MSG,
                MoveWindow, NONCLIENTMETRICSW, PostMessageW, PostQuitMessage, RegisterClassW,
                SPI_GETNONCLIENTMETRICS, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SendMessageW, SetForegroundWindow,
                SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, SystemParametersInfoW,
                TPM_LEFTBUTTON, TPM_RETURNCMD, TPM_RIGHTALIGN, TPM_TOPALIGN, TrackPopupMenuEx,
                TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND,
                WM_CTLCOLORSTATIC, WM_DESTROY, WM_DPICHANGED, WM_DRAWITEM, WM_ERASEBKGND,
                WM_GETMINMAXINFO, WM_KEYDOWN, WM_NCCREATE, WM_PAINT, WM_SETFONT, WM_SETTINGCHANGE,
                WM_SIZE, WM_THEMECHANGED, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN,
                WS_EX_CONTROLPARENT, WS_EX_DLGMODALFRAME, WS_EX_TOPMOST, WS_MAXIMIZEBOX,
                WS_OVERLAPPED, WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_VISIBLE,
            },
        },
    },
    core::{
        ComObject, Error as WindowsError, HRESULT, IInspectable, Interface, PCWSTR, PWSTR, Ref,
        Result as WindowsResult, implement, w,
    },
};

use crate::{
    folder_picker::{FolderPickerSelection, FolderPickerViewLabels, FolderPickerViewSettings},
    selection::VIDEO_EXTENSIONS,
};

const WINDOW_CLASS: PCWSTR = w!("VidmetryExplorerFolderPicker");
const BUTTON_CLASS: PCWSTR = w!("BUTTON");
const STATIC_CLASS: PCWSTR = w!("STATIC");

const ID_BACK: i32 = 1001;
const ID_UP: i32 = 1002;
const ID_SELECT: i32 = 1003;
const ID_CANCEL: i32 = 1004;
const ID_FORWARD: i32 = 1005;
const ID_PATH: i32 = 1006;
const ID_VIEW: i32 = 1007;
const ID_BROWSER_HOST: i32 = 1008;

const ID_VIEW_EXTRA_LARGE_ICONS: i32 = 1101;
const ID_VIEW_LARGE_ICONS: i32 = 1102;
const ID_VIEW_MEDIUM_ICONS: i32 = 1103;
const ID_VIEW_SMALL_ICONS: i32 = 1104;
const ID_VIEW_LIST: i32 = 1105;
const ID_VIEW_DETAILS: i32 = 1106;
const ID_VIEW_TILES: i32 = 1107;
const ID_VIEW_CONTENT: i32 = 1108;

const WM_NAVIGATION_COMPLETE: u32 = WM_APP + 1;
const WM_COLOR_MODE_CHANGED: u32 = WM_APP + 2;

const JAPANESE_PRIMARY_LANGUAGE_ID: u16 = 0x11;
const BASE_DPI: i32 = 96;
const INITIAL_WIDTH: i32 = 980;
const INITIAL_HEIGHT: i32 = 700;
const MINIMUM_WIDTH: i32 = 720;
const MINIMUM_HEIGHT: i32 = 500;
const MARGIN: i32 = 12;
const CONTROL_HEIGHT: i32 = 36;
const SMALL_BUTTON_WIDTH: i32 = 36;
const ACTION_BUTTON_WIDTH: i32 = 112;
const CONTROL_GAP: i32 = 4;
const COMMAND_BAR_HEIGHT: i32 = 40;
const VIEW_BUTTON_WIDTH: i32 = 48;
const SHELL_STATUS_CLIP_HEIGHT: i32 = 26;
const BREADCRUMB_HORIZONTAL_PADDING: i32 = 8;
const BREADCRUMB_SEPARATOR_WIDTH: i32 = 16;

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

#[implement(
    IServiceProvider,
    ICommDlgBrowser2,
    IExplorerBrowserEvents,
    IExplorerPaneVisibility
)]
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
        let service = unsafe { *guid_service };
        if service != ICommDlgBrowser::IID && service != IExplorerPaneVisibility::IID {
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

impl IExplorerPaneVisibility_Impl for ExplorerSite_Impl {
    fn GetPaneState(&self, pane: *const windows::core::GUID) -> WindowsResult<u32> {
        if pane.is_null() {
            return Err(WindowsError::from_hresult(E_POINTER));
        }
        let pane = unsafe { *pane };
        let state = if pane == EP_NavPane {
            EPS_DEFAULT_ON.0 | EPS_FORCE.0
        } else if pane == EP_Commands
            || pane == EP_Commands_Organize
            || pane == EP_Commands_View
            || pane == EP_DetailsPane
            || pane == EP_PreviewPane
            || pane == EP_QueryPane
            || pane == EP_AdvQueryPane
            || pane == EP_Ribbon
            || pane == EP_StatusBar
        {
            EPS_DEFAULT_OFF.0 | EPS_FORCE.0
        } else {
            EPS_DEFAULT_OFF.0
        };
        Ok(state as u32)
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

struct ViewLabelBuffers {
    view: Vec<u16>,
    extra_large_icons: Vec<u16>,
    large_icons: Vec<u16>,
    medium_icons: Vec<u16>,
    small_icons: Vec<u16>,
    list: Vec<u16>,
    details: Vec<u16>,
    tiles: Vec<u16>,
    content: Vec<u16>,
}

impl From<FolderPickerViewLabels> for ViewLabelBuffers {
    fn from(labels: FolderPickerViewLabels) -> Self {
        Self {
            view: wide_string(&labels.view),
            extra_large_icons: wide_string(&labels.extra_large_icons),
            large_icons: wide_string(&labels.large_icons),
            medium_icons: wide_string(&labels.medium_icons),
            small_icons: wide_string(&labels.small_icons),
            list: wide_string(&labels.list),
            details: wide_string(&labels.details),
            tiles: wide_string(&labels.tiles),
            content: wide_string(&labels.content),
        }
    }
}

struct PickerWindow {
    window: HWND,
    back_button: HWND,
    forward_button: HWND,
    up_button: HWND,
    path_box: HWND,
    view_button: HWND,
    tooltip: HWND,
    browser_host: HWND,
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
    selected: Option<FolderPickerSelection>,
    desired_view: Option<(FOLDERVIEWMODE, i32)>,
    breadcrumbs: Vec<BreadcrumbSegment>,
    select_folder_label: Vec<u16>,
    cancel_label: Vec<u16>,
    view_labels: ViewLabelBuffers,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BreadcrumbSegment {
    label: String,
    path: String,
}

#[derive(Clone, Copy)]
struct BreadcrumbPlacement {
    segment_index: usize,
    rect: RECT,
}

impl PickerWindow {
    fn new(
        select_folder_label: &str,
        cancel_label: &str,
        view_labels: FolderPickerViewLabels,
    ) -> Self {
        let palette = ThemePalette::fallback_light();
        Self {
            window: HWND::default(),
            back_button: HWND::default(),
            forward_button: HWND::default(),
            up_button: HWND::default(),
            path_box: HWND::default(),
            view_button: HWND::default(),
            tooltip: HWND::default(),
            browser_host: HWND::default(),
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
            selected: None,
            desired_view: None,
            breadcrumbs: Vec::new(),
            select_folder_label: wide_string(select_folder_label),
            cancel_label: wide_string(cancel_label),
            view_labels: view_labels.into(),
        }
    }

    fn initialize(
        &mut self,
        initial_directory: Option<&str>,
        initial_view_mode: Option<i32>,
        initial_icon_size: Option<i32>,
    ) -> WindowsResult<()> {
        self.initialize_color_mode_tracking();
        self.create_controls()?;
        self.desired_view = valid_view_settings(initial_view_mode, initial_icon_size)
            .map(|(mode, size)| (FOLDERVIEWMODE(mode), size));

        let browser: IExplorerBrowser =
            unsafe { CoCreateInstance(&ExplorerBrowser, None, CLSCTX_INPROC_SERVER)? };
        let site = ExplorerSite::new(self.window);
        let service_provider: IServiceProvider = site.to_interface();
        unsafe { IUnknown_SetSite(&browser, &service_provider)? };

        let settings = FOLDERSETTINGS {
            ViewMode: FVM_ICON.0 as u32,
            fFlags: (FWF_AUTOARRANGE | FWF_FULLROWSELECT | FWF_NOWEBVIEW).0 as u32,
        };
        let browser_rect = self.browser_inner_rect();
        unsafe {
            browser.Initialize(self.browser_host, &browser_rect, Some(&settings))?;
            browser.SetOptions(EBO_SHOWFRAMES | EBO_NOBORDER)?;
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
        self.apply_tooltip_theme();
        for control in [
            self.back_button,
            self.forward_button,
            self.up_button,
            self.path_box,
            self.view_button,
            self.browser_host,
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
        }
    }

    fn draw_control(&self, draw: &DRAWITEMSTRUCT) -> bool {
        if draw.hwndItem == self.path_box {
            return self.draw_path(draw);
        }
        if ![
            self.back_button,
            self.forward_button,
            self.up_button,
            self.view_button,
            self.select_button,
            self.cancel_button,
        ]
        .contains(&draw.hwndItem)
        {
            return false;
        }

        let has_state = |state: u32| draw.itemState.0 & state != 0;
        let is_navigation = [
            self.back_button,
            self.forward_button,
            self.up_button,
            self.view_button,
        ]
        .contains(&draw.hwndItem);
        let brush = if has_state(ODS_SELECTED.0) {
            self.brushes.pressed
        } else if has_state(ODS_HOTLIGHT.0) {
            self.brushes.hover
        } else if is_navigation {
            self.brushes.background
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
            if !is_navigation {
                FrameRect(draw.hDC, &draw.rcItem, border);
            }
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
        if is_navigation {
            if draw.hwndItem == self.view_button {
                self.draw_view_icon(draw);
            } else {
                self.draw_navigation_icon(draw);
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
            return true;
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

    fn draw_navigation_icon(&self, draw: &DRAWITEMSTRUCT) {
        let dpi = unsafe { GetDpiForWindow(self.window) } as i32;
        let radius = (5 * dpi / BASE_DPI).max(5);
        let line_width = (2 * dpi / BASE_DPI).max(1);
        let mut center_x = (draw.rcItem.left + draw.rcItem.right) / 2;
        let mut center_y = (draw.rcItem.top + draw.rcItem.bottom) / 2;
        if draw.itemState.0 & ODS_SELECTED.0 != 0 {
            center_x += 1;
            center_y += 1;
        }
        let pen = unsafe { CreatePen(PS_SOLID, line_width, self.palette.text) };
        if pen.is_invalid() {
            return;
        }
        let previous_pen = unsafe { SelectObject(draw.hDC, pen.into()) };
        unsafe {
            if draw.hwndItem == self.back_button {
                let _ = MoveToEx(draw.hDC, center_x + radius, center_y, None);
                let _ = LineTo(draw.hDC, center_x - radius, center_y);
                let _ = LineTo(draw.hDC, center_x, center_y - radius);
                let _ = MoveToEx(draw.hDC, center_x - radius, center_y, None);
                let _ = LineTo(draw.hDC, center_x, center_y + radius);
            } else if draw.hwndItem == self.forward_button {
                let _ = MoveToEx(draw.hDC, center_x - radius, center_y, None);
                let _ = LineTo(draw.hDC, center_x + radius, center_y);
                let _ = LineTo(draw.hDC, center_x, center_y - radius);
                let _ = MoveToEx(draw.hDC, center_x + radius, center_y, None);
                let _ = LineTo(draw.hDC, center_x, center_y + radius);
            } else {
                let _ = MoveToEx(draw.hDC, center_x, center_y + radius, None);
                let _ = LineTo(draw.hDC, center_x, center_y - radius);
                let _ = LineTo(draw.hDC, center_x - radius, center_y);
                let _ = MoveToEx(draw.hDC, center_x, center_y - radius, None);
                let _ = LineTo(draw.hDC, center_x + radius, center_y);
            }
            let _ = SelectObject(draw.hDC, previous_pen);
            let _ = DeleteObject(pen.into());
        }
    }

    fn draw_view_icon(&self, draw: &DRAWITEMSTRUCT) {
        let dpi = unsafe { GetDpiForWindow(self.window) } as i32;
        let scale = |value: i32| (value * dpi / BASE_DPI).max(1);
        let mut center_x = (draw.rcItem.left + draw.rcItem.right) / 2 - scale(4);
        let mut center_y = (draw.rcItem.top + draw.rcItem.bottom) / 2;
        if draw.itemState.0 & ODS_SELECTED.0 != 0 {
            center_x += 1;
            center_y += 1;
        }
        let pen = unsafe { CreatePen(PS_SOLID, scale(1), self.palette.text) };
        if pen.is_invalid() {
            return;
        }
        let icon_brush = unsafe { CreateSolidBrush(self.palette.text) };
        if icon_brush.is_invalid() {
            let _ = unsafe { DeleteObject(pen.into()) };
            return;
        }
        let previous_pen = unsafe { SelectObject(draw.hDC, pen.into()) };
        let square = scale(3);
        let line_left = center_x - scale(8);
        let line_right = center_x + scale(6);
        unsafe {
            for offset in [-scale(6), 0, scale(6)] {
                let top = center_y + offset - square / 2;
                let left = line_left;
                let rect = RECT {
                    left,
                    top,
                    right: left + square,
                    bottom: top + square,
                };
                FrameRect(draw.hDC, &rect, icon_brush);
                let _ = MoveToEx(draw.hDC, left + square + scale(3), center_y + offset, None);
                let _ = LineTo(draw.hDC, line_right, center_y + offset);
            }
            let chevron_x = center_x + scale(14);
            let chevron_y = center_y - scale(1);
            let _ = MoveToEx(draw.hDC, chevron_x - scale(3), chevron_y, None);
            let _ = LineTo(draw.hDC, chevron_x, chevron_y + scale(3));
            let _ = LineTo(draw.hDC, chevron_x + scale(3), chevron_y);
            let _ = SelectObject(draw.hDC, previous_pen);
            let _ = DeleteObject(pen.into());
            let _ = DeleteObject(icon_brush.into());
        }
    }

    fn draw_path(&self, draw: &DRAWITEMSTRUCT) -> bool {
        let has_state = |state: u32| draw.itemState.0 & state != 0;
        unsafe {
            FillRect(
                draw.hDC,
                &draw.rcItem,
                if has_state(ODS_SELECTED.0) {
                    self.brushes.pressed
                } else if has_state(ODS_HOTLIGHT.0) {
                    self.brushes.hover
                } else {
                    self.brushes.surface
                },
            );
            FrameRect(
                draw.hDC,
                &draw.rcItem,
                if has_state(ODS_FOCUS.0) {
                    self.brushes.accent
                } else {
                    self.brushes.border
                },
            );
            SetBkMode(draw.hDC, TRANSPARENT);
            SetTextColor(draw.hDC, self.palette.text);
        }
        let previous_font = self
            .font
            .map(|font| unsafe { SelectObject(draw.hDC, font.into()) });
        let (overflow, placements) = self.breadcrumb_placements(draw.hDC, draw.rcItem);
        let dpi = unsafe { GetDpiForWindow(self.window) } as i32;
        let scale = |value: i32| (value * dpi / BASE_DPI).max(1);
        let separator_width = scale(BREADCRUMB_SEPARATOR_WIDTH);
        let mut cursor = draw.rcItem.left + scale(BREADCRUMB_HORIZONTAL_PADDING);
        if overflow {
            let overflow_width = self.breadcrumb_text_width(draw.hDC, "…")
                + scale(BREADCRUMB_HORIZONTAL_PADDING) * 2;
            let mut overflow_rect = RECT {
                left: cursor,
                top: draw.rcItem.top,
                right: cursor + overflow_width,
                bottom: draw.rcItem.bottom,
            };
            let mut overflow_text = "…".encode_utf16().collect::<Vec<_>>();
            unsafe {
                DrawTextW(
                    draw.hDC,
                    &mut overflow_text,
                    &mut overflow_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
            cursor += overflow_width;
        }
        for (placement_index, placement) in placements.iter().enumerate() {
            if overflow || placement_index > 0 {
                let mut separator_rect = RECT {
                    left: cursor,
                    top: draw.rcItem.top,
                    right: cursor + separator_width,
                    bottom: draw.rcItem.bottom,
                };
                let mut separator = ">".encode_utf16().collect::<Vec<_>>();
                unsafe {
                    DrawTextW(
                        draw.hDC,
                        &mut separator,
                        &mut separator_rect,
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                    );
                }
            }
            let segment = &self.breadcrumbs[placement.segment_index];
            let mut text = segment.label.encode_utf16().collect::<Vec<_>>();
            let mut text_rect = placement.rect;
            unsafe {
                DrawTextW(
                    draw.hDC,
                    &mut text,
                    &mut text_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
                );
            }
            cursor = placement.rect.right;
        }
        if let Some(previous_font) = previous_font {
            let _ = unsafe { SelectObject(draw.hDC, previous_font) };
        }
        if has_state(ODS_FOCUS.0) {
            let mut focus = draw.rcItem;
            let inset = scale(3);
            focus.left += inset;
            focus.top += inset;
            focus.right -= inset;
            focus.bottom -= inset;
            let _ = unsafe { DrawFocusRect(draw.hDC, &focus) };
        }
        true
    }

    fn breadcrumb_text_width(&self, dc: HDC, text: &str) -> i32 {
        let mut text = text.encode_utf16().collect::<Vec<_>>();
        let mut bounds = RECT::default();
        unsafe {
            DrawTextW(
                dc,
                &mut text,
                &mut bounds,
                DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
        bounds.right.max(1)
    }

    fn breadcrumb_placements(&self, dc: HDC, bounds: RECT) -> (bool, Vec<BreadcrumbPlacement>) {
        if self.breadcrumbs.is_empty() {
            return (false, Vec::new());
        }
        let dpi = unsafe { GetDpiForWindow(self.window) } as i32;
        let scale = |value: i32| (value * dpi / BASE_DPI).max(1);
        let outer_padding = scale(BREADCRUMB_HORIZONTAL_PADDING);
        let text_padding = outer_padding;
        let separator_width = scale(BREADCRUMB_SEPARATOR_WIDTH);
        let available_width = (bounds.right - bounds.left - outer_padding * 2).max(1);
        let widths = self
            .breadcrumbs
            .iter()
            .map(|segment| self.breadcrumb_text_width(dc, &segment.label) + text_padding * 2)
            .collect::<Vec<_>>();
        let overflow_width =
            self.breadcrumb_text_width(dc, "…") + text_padding * 2 + separator_width;
        let start =
            visible_breadcrumb_start(&widths, available_width, separator_width, overflow_width);
        let overflow = start > 0;
        let mut cursor = bounds.left + outer_padding;
        if overflow {
            cursor += overflow_width - separator_width;
        }
        let mut placements = Vec::with_capacity(self.breadcrumbs.len() - start);
        for (visible_index, segment_index) in (start..self.breadcrumbs.len()).enumerate() {
            if overflow || visible_index > 0 {
                cursor += separator_width;
            }
            let width = widths[segment_index].min((bounds.right - outer_padding - cursor).max(1));
            placements.push(BreadcrumbPlacement {
                segment_index,
                rect: RECT {
                    left: cursor,
                    top: bounds.top,
                    right: cursor + width,
                    bottom: bounds.bottom,
                },
            });
            cursor += width;
        }
        (overflow, placements)
    }

    fn browser_host_color(&self, dc: HDC) -> LRESULT {
        unsafe {
            SetBkMode(dc, OPAQUE);
            SetBkColor(dc, self.palette.background);
        }
        LRESULT(self.brushes.background.0 as isize)
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
        self.forward_button = create_control(
            BUTTON_CLASS,
            w!("\u{2192}"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            WINDOW_EX_STYLE(0),
            self.window,
            ID_FORWARD,
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
            BUTTON_CLASS,
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            WINDOW_EX_STYLE(0),
            self.window,
            ID_PATH,
            instance,
        )?;
        self.view_button = create_control(
            BUTTON_CLASS,
            PCWSTR(self.view_labels.view.as_ptr()),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            WINDOW_EX_STYLE(0),
            self.window,
            ID_VIEW,
            instance,
        )?;
        self.create_view_tooltip(instance)?;
        self.browser_host = create_control(
            STATIC_CLASS,
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
            WINDOW_EX_STYLE(0),
            self.window,
            ID_BROWSER_HOST,
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
                self.forward_button,
                self.up_button,
                self.path_box,
                self.view_button,
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

    fn create_view_tooltip(&mut self, instance: HINSTANCE) -> WindowsResult<()> {
        self.tooltip = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST,
                TOOLTIPS_CLASS,
                PCWSTR::null(),
                WS_POPUP | WINDOW_STYLE(TTS_ALWAYSTIP | TTS_NOPREFIX),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                Some(self.window),
                None,
                Some(instance),
                None,
            )?
        };
        let tool = TTTOOLINFOW {
            cbSize: std::mem::size_of::<TTTOOLINFOW>() as u32,
            uFlags: TTF_IDISHWND | TTF_SUBCLASS,
            hwnd: self.window,
            uId: self.view_button.0 as usize,
            lpszText: PWSTR(self.view_labels.view.as_mut_ptr()),
            ..Default::default()
        };
        unsafe {
            SendMessageW(
                self.tooltip,
                TTM_ADDTOOLW,
                Some(WPARAM(0)),
                Some(LPARAM((&tool as *const TTTOOLINFOW) as isize)),
            );
        }
        self.apply_tooltip_theme();
        Ok(())
    }

    fn apply_tooltip_theme(&self) {
        if self.tooltip.0.is_null() {
            return;
        }
        let theme = if self.palette.dark {
            w!("DarkMode_Explorer")
        } else {
            w!("Explorer")
        };
        let _ = unsafe { SetWindowTheme(self.tooltip, theme, PCWSTR::null()) };
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
                self.forward_button,
                margin + small_width + gap,
                top,
                small_width,
                control_height,
                true,
            )
        };
        let _ = unsafe {
            MoveWindow(
                self.up_button,
                margin + (small_width + gap) * 2,
                top,
                small_width,
                control_height,
                true,
            )
        };
        let path_left = margin + (small_width + gap) * 3;
        let path_width = (width - margin - path_left).max(1);
        let _ = unsafe {
            MoveWindow(
                self.path_box,
                path_left,
                top,
                path_width,
                control_height,
                true,
            )
        };
        let command_top = top + control_height + gap;
        let view_width = scale(VIEW_BUTTON_WIDTH);
        let _ = unsafe {
            MoveWindow(
                self.view_button,
                width - margin - view_width,
                command_top + (scale(COMMAND_BAR_HEIGHT) - control_height) / 2,
                view_width,
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

        let host_rect = self.browser_host_rect();
        let _ = unsafe {
            MoveWindow(
                self.browser_host,
                host_rect.left,
                host_rect.top,
                host_rect.right - host_rect.left,
                host_rect.bottom - host_rect.top,
                true,
            )
        };
        if let Some(browser) = &self.browser {
            let _ = unsafe { browser.SetRect(None, self.browser_inner_rect()) };
        }
    }

    fn browser_host_rect(&self) -> RECT {
        let dpi = unsafe { GetDpiForWindow(self.window) } as i32;
        let scale = |value| value * dpi / BASE_DPI;
        let mut client = RECT::default();
        let _ = unsafe { GetClientRect(self.window, &mut client) };
        RECT {
            left: scale(MARGIN),
            top: scale(MARGIN + CONTROL_HEIGHT + CONTROL_GAP + COMMAND_BAR_HEIGHT + CONTROL_GAP),
            right: client.right - scale(MARGIN),
            bottom: client.bottom - scale(MARGIN + CONTROL_HEIGHT + CONTROL_GAP),
        }
    }

    fn browser_inner_rect(&self) -> RECT {
        let dpi = unsafe { GetDpiForWindow(self.window) } as i32;
        let mut client = RECT::default();
        let _ = unsafe { GetClientRect(self.browser_host, &mut client) };
        client.bottom += SHELL_STATUS_CLIP_HEIGHT * dpi / BASE_DPI;
        client
    }

    fn navigate(&self, flags: u32) {
        if let Some(browser) = &self.browser {
            let _ = unsafe { browser.BrowseToIDList(null(), flags) };
        }
    }

    fn update_current_folder(&mut self) {
        self.apply_desired_view();
        let path = self.current_folder_path();
        self.breadcrumbs = path.as_deref().map(breadcrumb_segments).unwrap_or_default();
        let label = wide_string(path.as_deref().unwrap_or(""));
        let _ = unsafe { SetWindowTextW(self.path_box, PCWSTR(label.as_ptr())) };
        let _ = unsafe {
            RedrawWindow(
                Some(self.path_box),
                None,
                None,
                RDW_INVALIDATE | RDW_UPDATENOW,
            )
        };
        let _ = unsafe { EnableWindow(self.select_button, path.is_some()) };
    }

    fn navigate_breadcrumb_at_cursor(&self) {
        let mut point = POINT::default();
        if unsafe { GetCursorPos(&mut point) }.is_err()
            || !unsafe { ScreenToClient(self.path_box, &mut point) }.as_bool()
        {
            return;
        }
        let dc = unsafe { GetDC(Some(self.path_box)) };
        if dc.is_invalid() {
            return;
        }
        let previous_font = self
            .font
            .map(|font| unsafe { SelectObject(dc, font.into()) });
        let mut bounds = RECT::default();
        let placements = if unsafe { GetClientRect(self.path_box, &mut bounds) }.is_ok() {
            self.breadcrumb_placements(dc, bounds).1
        } else {
            Vec::new()
        };
        if let Some(previous_font) = previous_font {
            let _ = unsafe { SelectObject(dc, previous_font) };
        }
        let _ = unsafe { ReleaseDC(Some(self.path_box), dc) };

        let target = placements
            .iter()
            .find(|placement| point_in_rect(point, placement.rect))
            .and_then(|placement| self.breadcrumbs.get(placement.segment_index))
            .map(|segment| segment.path.clone());
        let Some(target) = target else {
            return;
        };
        let Ok(item) = shell_item_from_path(&target) else {
            return;
        };
        if let Some(browser) = &self.browser {
            let _ = unsafe { browser.BrowseToObject(&item, 0) };
        }
    }

    fn apply_desired_view(&self) {
        let Some((view_mode, icon_size)) = self.desired_view else {
            return;
        };
        let Some(browser) = &self.browser else {
            return;
        };
        let Ok(view) = (unsafe { browser.GetCurrentView::<IFolderView2>() }) else {
            return;
        };
        let _ = unsafe { view.SetViewModeAndIconSize(view_mode, icon_size) };
    }

    fn current_folder_path(&self) -> Option<String> {
        let browser = self.browser.as_ref()?;
        let view: IFolderView2 = unsafe { browser.GetCurrentView() }.ok()?;
        let item: IShellItem = unsafe { view.GetFolder() }.ok()?;
        shell_item_path(&item).ok()
    }

    fn current_view_settings(&self) -> Option<(i32, i32)> {
        let browser = self.browser.as_ref()?;
        let view: IFolderView2 = unsafe { browser.GetCurrentView() }.ok()?;
        let mut view_mode = FOLDERVIEWMODE::default();
        let mut icon_size = 0;
        unsafe { view.GetViewModeAndIconSize(&mut view_mode, &mut icon_size) }.ok()?;
        valid_view_settings(Some(view_mode.0), Some(icon_size))
    }

    fn set_view(&mut self, view_mode: FOLDERVIEWMODE, icon_size: i32) {
        let Some(browser) = &self.browser else {
            return;
        };
        let Ok(view) = (unsafe { browser.GetCurrentView::<IFolderView2>() }) else {
            return;
        };
        if unsafe { view.SetViewModeAndIconSize(view_mode, icon_size) }.is_ok() {
            self.desired_view = Some((view_mode, icon_size));
        }
    }

    fn show_view_menu(&mut self) {
        let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
            return;
        };
        let checked = self
            .current_view_settings()
            .map(|(mode, size)| view_command_for_settings(FOLDERVIEWMODE(mode), size));
        let items = [
            (
                ID_VIEW_EXTRA_LARGE_ICONS,
                &self.view_labels.extra_large_icons,
            ),
            (ID_VIEW_LARGE_ICONS, &self.view_labels.large_icons),
            (ID_VIEW_MEDIUM_ICONS, &self.view_labels.medium_icons),
            (ID_VIEW_SMALL_ICONS, &self.view_labels.small_icons),
            (ID_VIEW_LIST, &self.view_labels.list),
            (ID_VIEW_DETAILS, &self.view_labels.details),
            (ID_VIEW_TILES, &self.view_labels.tiles),
            (ID_VIEW_CONTENT, &self.view_labels.content),
        ];
        for (command, label) in items {
            let flags = if checked == Some(command) {
                MF_STRING | MF_CHECKED
            } else {
                MF_STRING
            };
            if unsafe { AppendMenuW(menu, flags, command as usize, PCWSTR(label.as_ptr())) }
                .is_err()
            {
                let _ = unsafe { DestroyMenu(menu) };
                return;
            }
        }

        let mut button = RECT::default();
        if unsafe { GetWindowRect(self.view_button, &mut button) }.is_err() {
            let _ = unsafe { DestroyMenu(menu) };
            return;
        }
        let flags = (TPM_RIGHTALIGN | TPM_TOPALIGN | TPM_RETURNCMD | TPM_LEFTBUTTON).0;
        let command = unsafe {
            TrackPopupMenuEx(menu, flags, button.right, button.bottom, self.window, None).0
        };
        let _ = unsafe { DestroyMenu(menu) };
        if let Some((view_mode, icon_size)) = view_settings_for_command(command) {
            self.set_view(view_mode, icon_size);
        }
    }

    fn accept(&mut self) {
        if let Some(path) = self.current_folder_path() {
            let (view_mode, icon_size) = self
                .current_view_settings()
                .or_else(|| self.desired_view.map(|(mode, size)| (mode.0, size)))
                .unwrap_or((FVM_ICON.0, 96));
            self.selected = Some(FolderPickerSelection {
                path,
                view_mode,
                icon_size,
            });
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
            if draw.is_some_and(|draw| state.draw_control(draw)) {
                LRESULT(1)
            } else {
                unsafe { DefWindowProcW(window, message, wparam, lparam) }
            }
        }
        WM_CTLCOLORSTATIC => {
            let control = HWND(lparam.0 as *mut c_void);
            if control == state.browser_host {
                state.browser_host_color(HDC(wparam.0 as *mut c_void))
            } else {
                unsafe { DefWindowProcW(window, message, wparam, lparam) }
            }
        }
        WM_SIZE => {
            state.layout();
            LRESULT(0)
        }
        WM_COMMAND => {
            let command = (wparam.0 & 0xffff) as i32;
            if let Some(flags) = navigation_flags_for_command(command) {
                state.navigate(flags);
            } else {
                match command {
                    ID_PATH => state.navigate_breadcrumb_at_cursor(),
                    ID_VIEW => state.show_view_menu(),
                    ID_SELECT => state.accept(),
                    ID_CANCEL => {
                        let _ = unsafe { DestroyWindow(window) };
                    }
                    _ => {}
                }
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
    initial_view: FolderPickerViewSettings,
    view_labels: FolderPickerViewLabels,
) -> Result<Option<FolderPickerSelection>, String> {
    let initialization =
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    initialization.ok().map_err(windows_error)?;
    let _com_apartment = ComApartment;
    unsafe { RoInitialize(RO_INIT_SINGLETHREADED) }.map_err(windows_error)?;
    let _winrt_apartment = WinRtApartment;

    register_window_class().map_err(windows_error)?;
    let owner = HWND(owner as *mut c_void);
    let mut picker = Box::new(PickerWindow::new(
        select_folder_label,
        cancel_label,
        view_labels,
    ));
    let window = create_picker_window(owner, title, picker.as_mut()).map_err(windows_error)?;

    if let Err(error) = picker.initialize(
        initial_directory,
        Some(initial_view.view_mode),
        Some(initial_view.icon_size),
    ) {
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
    Ok(picker.selected.clone())
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

fn breadcrumb_segments(path: &str) -> Vec<BreadcrumbSegment> {
    let mut ancestors = Path::new(path)
        .ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .map(|ancestor| {
            let label = ancestor
                .file_name()
                .filter(|name| !name.is_empty())
                .unwrap_or(ancestor.as_os_str())
                .to_string_lossy()
                .into_owned();
            BreadcrumbSegment {
                label,
                path: ancestor.to_string_lossy().into_owned(),
            }
        })
        .collect::<Vec<_>>();
    ancestors.reverse();
    ancestors
}

fn visible_breadcrumb_start(
    widths: &[i32],
    available_width: i32,
    separator_width: i32,
    overflow_width: i32,
) -> usize {
    if widths.is_empty() {
        return 0;
    }
    let complete_width =
        widths.iter().sum::<i32>() + separator_width * (widths.len().saturating_sub(1) as i32);
    if complete_width <= available_width {
        return 0;
    }

    let mut start = widths.len() - 1;
    let mut used = overflow_width + widths[start];
    while start > 0 {
        let candidate = used + separator_width + widths[start - 1];
        if candidate > available_width {
            break;
        }
        start -= 1;
        used = candidate;
    }
    start.max(1)
}

fn point_in_rect(point: POINT, rect: RECT) -> bool {
    point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
}

fn valid_view_settings(view_mode: Option<i32>, icon_size: Option<i32>) -> Option<(i32, i32)> {
    let view_mode = view_mode?;
    let icon_size = icon_size?;
    ((1..=8).contains(&view_mode) && (16..=512).contains(&icon_size))
        .then_some((view_mode, icon_size))
}

fn view_settings_for_command(command: i32) -> Option<(FOLDERVIEWMODE, i32)> {
    match command {
        ID_VIEW_EXTRA_LARGE_ICONS => Some((FVM_ICON, 256)),
        ID_VIEW_LARGE_ICONS => Some((FVM_ICON, 96)),
        ID_VIEW_MEDIUM_ICONS => Some((FVM_ICON, 48)),
        ID_VIEW_SMALL_ICONS => Some((FVM_SMALLICON, 16)),
        ID_VIEW_LIST => Some((FVM_LIST, 16)),
        ID_VIEW_DETAILS => Some((FVM_DETAILS, 16)),
        ID_VIEW_TILES => Some((FVM_TILE, 48)),
        ID_VIEW_CONTENT => Some((FVM_CONTENT, 32)),
        _ => None,
    }
}

fn navigation_flags_for_command(command: i32) -> Option<u32> {
    match command {
        ID_BACK => Some(SBSP_NAVIGATEBACK),
        ID_UP => Some(SBSP_PARENT),
        ID_FORWARD => Some(SBSP_NAVIGATEFORWARD),
        _ => None,
    }
}

fn view_command_for_settings(view_mode: FOLDERVIEWMODE, icon_size: i32) -> i32 {
    match view_mode {
        FVM_ICON | FVM_THUMBNAIL if icon_size >= 192 => ID_VIEW_EXTRA_LARGE_ICONS,
        FVM_ICON | FVM_THUMBNAIL if icon_size >= 72 => ID_VIEW_LARGE_ICONS,
        FVM_ICON | FVM_THUMBNAIL => ID_VIEW_MEDIUM_ICONS,
        FVM_SMALLICON => ID_VIEW_SMALL_ICONS,
        FVM_LIST => ID_VIEW_LIST,
        FVM_DETAILS => ID_VIEW_DETAILS,
        FVM_TILE => ID_VIEW_TILES,
        FVM_CONTENT => ID_VIEW_CONTENT,
        _ => ID_VIEW_MEDIUM_ICONS,
    }
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

    #[test]
    fn explorer_view_settings_accept_only_shell_modes_and_practical_icon_sizes() {
        assert_eq!(valid_view_settings(Some(1), Some(16)), Some((1, 16)));
        assert_eq!(valid_view_settings(Some(8), Some(512)), Some((8, 512)));
        assert_eq!(valid_view_settings(Some(0), Some(96)), None);
        assert_eq!(valid_view_settings(Some(9), Some(96)), None);
        assert_eq!(valid_view_settings(Some(5), Some(15)), None);
        assert_eq!(valid_view_settings(Some(5), Some(513)), None);
        assert_eq!(valid_view_settings(None, Some(96)), None);
    }

    #[test]
    fn explorer_view_menu_maps_every_supported_display_mode() {
        let cases = [
            (ID_VIEW_EXTRA_LARGE_ICONS, FVM_ICON, 256),
            (ID_VIEW_LARGE_ICONS, FVM_ICON, 96),
            (ID_VIEW_MEDIUM_ICONS, FVM_ICON, 48),
            (ID_VIEW_SMALL_ICONS, FVM_SMALLICON, 16),
            (ID_VIEW_LIST, FVM_LIST, 16),
            (ID_VIEW_DETAILS, FVM_DETAILS, 16),
            (ID_VIEW_TILES, FVM_TILE, 48),
            (ID_VIEW_CONTENT, FVM_CONTENT, 32),
        ];
        for (command, mode, size) in cases {
            assert_eq!(view_settings_for_command(command), Some((mode, size)));
            assert_eq!(view_command_for_settings(mode, size), command);
        }
        assert_eq!(
            view_command_for_settings(FVM_THUMBNAIL, 256),
            ID_VIEW_EXTRA_LARGE_ICONS
        );
        assert_eq!(
            view_command_for_settings(FVM_THUMBNAIL, 96),
            ID_VIEW_LARGE_ICONS
        );
        assert_eq!(
            view_command_for_settings(FVM_THUMBNAIL, 48),
            ID_VIEW_MEDIUM_ICONS
        );
        assert_eq!(view_settings_for_command(9999), None);
    }

    #[test]
    fn picker_navigation_keeps_history_and_parent_actions_distinct() {
        assert_eq!(
            navigation_flags_for_command(ID_BACK),
            Some(SBSP_NAVIGATEBACK)
        );
        assert_eq!(navigation_flags_for_command(ID_UP), Some(SBSP_PARENT));
        assert_eq!(
            navigation_flags_for_command(ID_FORWARD),
            Some(SBSP_NAVIGATEFORWARD)
        );
        assert_eq!(navigation_flags_for_command(ID_VIEW), None);
    }

    #[test]
    fn folder_paths_are_exposed_as_clickable_breadcrumb_targets() {
        assert_eq!(
            breadcrumb_segments(r"C:\Users\dwarf\Videos"),
            vec![
                BreadcrumbSegment {
                    label: String::from(r"C:\"),
                    path: String::from(r"C:\"),
                },
                BreadcrumbSegment {
                    label: String::from("Users"),
                    path: String::from(r"C:\Users"),
                },
                BreadcrumbSegment {
                    label: String::from("dwarf"),
                    path: String::from(r"C:\Users\dwarf"),
                },
                BreadcrumbSegment {
                    label: String::from("Videos"),
                    path: String::from(r"C:\Users\dwarf\Videos"),
                },
            ]
        );
    }

    #[test]
    fn long_breadcrumbs_keep_the_current_and_nearest_parent_visible() {
        assert_eq!(visible_breadcrumb_start(&[40, 50, 60], 200, 16, 40), 0);
        assert_eq!(visible_breadcrumb_start(&[80, 80, 80], 190, 16, 40), 2);
        assert_eq!(visible_breadcrumb_start(&[80, 40, 40], 160, 16, 40), 1);
    }
}
