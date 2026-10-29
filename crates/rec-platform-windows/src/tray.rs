//! Windows System Tray (Notification Area) integration for Screen Recorder Pro.
//!
//! Conforms to `[SPEC-GUI-001]`, `[SPEC-PLAT-WIN-001]`, and Clean Architecture (DIP).
//!
//! # Architectural Overview
//!
//! Coordinates Win32 notification area interactions:
//! - **Shell Notification Icon:** Registers modern notification icon via [`Shell_NotifyIconW`].
//! - **Background Message Pump:** Dedicated worker thread running Win32 `GetMessageW` loop.
//! - **Synchronous Command Dispatching:** Employs [`TrackPopupMenu`] with `TPM_RETURNCMD` to
//!   synchronously intercept user clicks, eliminating message drops.
//! - **Native Window Restoration:** Restores and focuses the primary viewport via [`FindWindowW`]
//!   and [`ShowWindow`].
//! - **Explorer Watchdog:** Automatically resurrects the tray icon if Windows Explorer restarts
//!   (`TaskbarCreated` broadcast message) or drops icons.

use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use windows::core::{w, BOOL, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC,
    DeleteObject, Ellipse, GetDC, ReleaseDC, SelectObject,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NIM_SETVERSION, NIN_SELECT, NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, VK_F10, VK_F9,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyMenu, DestroyWindow, DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW,
    KillTimer, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassExW, RegisterWindowMessageW,
    SetForegroundWindow, SetTimer, ShowWindow, TrackPopupMenu, TranslateMessage, HICON,
    ICONINFO, IDI_APPLICATION, MF_SEPARATOR, MF_STRING, MSG, SW_RESTORE, SW_SHOW, TPM_BOTTOMALIGN,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY,
    WM_HOTKEY, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WM_TIMER, WNDCLASSEXW,
    WNDCLASS_STYLES, WS_EX_TOOLWINDOW, WS_POPUP,
};

/// Message identifier used for tray icon callbacks.
const WM_APP_TRAY_MSG: u32 = WM_APP + 201;

/// Global Hotkey identifier for F9 (Toggle Record).
pub const HOTKEY_ID_F9: i32 = 2001;
/// Global Hotkey identifier for F10 (Take Snapshot).
pub const HOTKEY_ID_F10: i32 = 2002;

/// Tray menu command identifiers: Open Main Window.
pub const IDM_OPEN: usize = 1001;
/// Tray menu command identifier: Start or Stop Recording.
pub const IDM_TOGGLE_RECORDING: usize = 1002;
/// Tray menu command identifier: Save Instant Replay buffer.
pub const IDM_INSTANT_REPLAY: usize = 1003;
/// Tray menu command identifier: Take Screenshot.
pub const IDM_TAKE_SNAPSHOT: usize = 1004;
/// Tray menu command identifier: Open Output Directory.
pub const IDM_OPEN_OUTPUT_DIR: usize = 1005;
/// Tray menu command identifier: Gracefully Exit Application.
pub const IDM_EXIT: usize = 1099;

/// Timer identifier used for periodic system tray icon presence heartbeat.
const IDT_TRAY_WATCHDOG: usize = 1001;

/// Modern Windows Shell Notification primary selection event (keyboard activation).
const NIN_KEYSELECT: u32 = NIN_SELECT + 1;

/// Global atomic message ID for the Windows Explorer "TaskbarCreated" broadcast.
static WM_TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

/// Global atomic storing the raw handle of the active system tray icon.
static TRAY_ICON_HANDLE: AtomicIsize = AtomicIsize::new(0);

/// Global atomic flag tracking whether an application shutdown has been requested.
static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Returns whether an application exit has been requested from the system tray or GUI.
#[must_use]
pub fn is_exit_requested() -> bool {
    EXIT_REQUESTED.load(Ordering::SeqCst)
}

/// Requests a graceful shutdown of the application.
pub fn request_app_exit() {
    EXIT_REQUESTED.store(true, Ordering::SeqCst);
    unsafe {
        let main_hwnd = FindWindowW(None, w!("Screen Recorder Pro"));
        if let Ok(hwnd) = main_hwnd {
            if !hwnd.0.is_null() {
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }
    // Safety watchdog: ensure termination if message loop is unresponsive
    thread::spawn(|| {
        thread::sleep(std::time::Duration::from_millis(800));
        if EXIT_REQUESTED.load(Ordering::SeqCst) {
            std::process::exit(0);
        }
    });
}

/// Restores and brings the main Screen Recorder Pro window to the foreground.
///
/// # Returns
/// `true` if the window was located and the show command was dispatched.
pub fn restore_main_window() -> bool {
    unsafe {
        let main_hwnd = FindWindowW(None, w!("Screen Recorder Pro"));
        if let Ok(hwnd) = main_hwnd {
            if !hwnd.0.is_null() {
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// Events dispatched from the system tray or global hotkeys to the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayEvent {
    /// Request to show and focus the main GUI window.
    ShowWindow,
    /// Request to toggle recording state (start or stop).
    ToggleRecording,
    /// Request to take a screenshot snapshot.
    TakeSnapshot,
    /// Request to open the destination recordings folder.
    OpenOutputDir,
    /// Request to save instant replay buffer.
    SaveInstantReplay,
    /// Request to completely terminate the application.
    ExitApp,
}

/// Windows System Tray Manager.
///
/// Encapsulates a persistent notification area icon, message pump,
/// and context menu running on a background thread.
#[derive(Debug)]
pub struct Win32TrayManager {
    events: Arc<Mutex<VecDeque<TrayEvent>>>,
    active: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
    window_hwnd: Arc<Mutex<Option<isize>>>,
}

impl Win32TrayManager {
    /// Creates a new [`Win32TrayManager`] with an observer wakeup callback.
    ///
    /// # Arguments
    /// * `wakeup` - Callback invoked on any tray action to trigger GUI repaints.
    ///
    /// # Errors
    /// Returns [`std::io::Error`] if spawning the tray thread fails.
    pub fn new_with_wakeup(
        wakeup: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, std::io::Error> {
        let events = Arc::new(Mutex::new(VecDeque::new()));
        let active = Arc::new(AtomicBool::new(true));
        let window_hwnd = Arc::new(Mutex::new(None));

        let t_events = Arc::clone(&events);
        let t_active = Arc::clone(&active);
        let t_hwnd = Arc::clone(&window_hwnd);
        let wakeup_arc = Arc::new(wakeup);

        let thread_handle = thread::Builder::new()
            .name("rec-system-tray".into())
            .spawn(move || {
                run_tray_thread(t_events, t_active, t_hwnd, wakeup_arc);
            })?;

        // Yield briefly for window and notification icon registration
        thread::sleep(std::time::Duration::from_millis(40));

        Ok(Self {
            events,
            active,
            thread_handle: Some(thread_handle),
            window_hwnd,
        })
    }

    /// Polls for the next pending event emitted by the tray icon or context menu.
    ///
    /// # Returns
    /// An optional [`TrayEvent`] if one is pending in the queue.
    pub fn poll_event(&self) -> Option<TrayEvent> {
        if let Ok(mut queue) = self.events.lock() {
            queue.pop_front()
        } else {
            None
        }
    }
}

impl Drop for Win32TrayManager {
    fn drop(&mut self) {
        self.active.store(false, Ordering::SeqCst);

        if let Ok(guard) = self.window_hwnd.lock() {
            if let Some(hwnd_raw) = *guard {
                let hwnd = HWND(hwnd_raw as *mut c_void);
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
            }
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Generates or loads the application icon for the Windows system tray.
fn create_custom_tray_icon() -> HICON {
    unsafe {
        // Attempt to load the branded embedded icon resource (ID 1)
        if let Ok(hinst) = windows::Win32::System::LibraryLoader::GetModuleHandleW(None) {
            #[allow(clippy::manual_dangling_ptr)]
            if let Ok(icon) = LoadIconW(Some(HINSTANCE(hinst.0)), PCWSTR(1 as *const u16)) {
                if !icon.0.is_null() {
                    return icon;
                }
            }
        }

        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            return LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
        }

        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        let color_bmp = CreateCompatibleBitmap(screen_dc, 16, 16);
        let mask_bmp = CreateBitmap(16, 16, 1, 1, None);

        let old_bmp = SelectObject(mem_dc, color_bmp.into());

        // Fill background with transparent mask area
        let bg_brush = CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x00000000));
        let red_brush = CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x002B30F4)); // Vibrant Red BGR (#F4302B)
        let white_brush = CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x00FFFFFF));

        let _ = windows::Win32::Graphics::Gdi::FillRect(
            mem_dc,
            &windows::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: 16,
                bottom: 16,
            },
            bg_brush,
        );

        // Draw outer red recording circle
        let _ = SelectObject(mem_dc, red_brush.into());
        let _ = Ellipse(mem_dc, 1, 1, 15, 15);

        // Draw central white highlight dot
        let _ = SelectObject(mem_dc, white_brush.into());
        let _ = Ellipse(mem_dc, 5, 5, 11, 11);

        SelectObject(mem_dc, old_bmp);

        let icon_info = ICONINFO {
            fIcon: BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask_bmp,
            hbmColor: color_bmp,
        };

        let hicon = CreateIconIndirect(&icon_info).unwrap_or_else(|_| {
            LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
        });

        let _ = DeleteObject(bg_brush.into());
        let _ = DeleteObject(red_brush.into());
        let _ = DeleteObject(white_brush.into());
        let _ = DeleteObject(color_bmp.into());
        let _ = DeleteObject(mask_bmp.into());
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);

        hicon
    }
}

/// Global static event queue accessed by window procedures and menu tracking.
static GLOBAL_EVENTS: Mutex<Option<Arc<Mutex<VecDeque<TrayEvent>>>>> = Mutex::new(None);
/// Global static observer callback used to wake up the GUI event loop.
static GLOBAL_WAKEUP: Mutex<Option<Arc<dyn Fn() + Send + Sync>>> = Mutex::new(None);

/// Dispatches a tray event to all registered sinks.
fn dispatch_tray_event(event: TrayEvent) {
    if matches!(event, TrayEvent::ShowWindow) {
        restore_main_window();
    } else if matches!(event, TrayEvent::ExitApp) {
        request_app_exit();
    }

    if let Ok(guard) = GLOBAL_EVENTS.lock() {
        if let Some(ref q) = *guard {
            if let Ok(mut events) = q.lock() {
                events.push_back(event.clone());
            }
        }
    }

    let wakeup_opt = if let Ok(guard) = GLOBAL_WAKEUP.lock() {
        guard.clone()
    } else {
        None
    };
    if let Some(wakeup) = wakeup_opt {
        wakeup();
    }
}

/// Maps a Win32 context menu command ID to its corresponding [`TrayEvent`].
///
/// # Arguments
///
/// * `cmd_id` - Numeric command identifier from the Win32 popup menu (`IDM_*`).
pub fn handle_menu_command(cmd_id: usize) {
    let event = match cmd_id {
        IDM_OPEN => Some(TrayEvent::ShowWindow),
        IDM_TOGGLE_RECORDING => Some(TrayEvent::ToggleRecording),
        IDM_TAKE_SNAPSHOT => Some(TrayEvent::TakeSnapshot),
        IDM_OPEN_OUTPUT_DIR => Some(TrayEvent::OpenOutputDir),
        IDM_INSTANT_REPLAY => Some(TrayEvent::SaveInstantReplay),
        IDM_EXIT => Some(TrayEvent::ExitApp),
        _ => None,
    };

    if let Some(ev) = event {
        dispatch_tray_event(ev);
    }
}

/// Verifies presence of the tray icon in Windows notification area and re-registers it if missing.
unsafe fn ensure_tray_icon_active(hwnd: HWND) {
    let icon_raw = TRAY_ICON_HANDLE.load(Ordering::Relaxed);
    if icon_raw == 0 {
        return;
    }
    let hicon = HICON(icon_raw as *mut c_void);
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 100,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
        uCallbackMessage: WM_APP_TRAY_MSG,
        hIcon: hicon,
        szTip: [0u16; 128],
        ..Default::default()
    };

    let tooltip = "Screen Recorder Pro";
    for (i, c) in tooltip.encode_utf16().enumerate().take(127) {
        nid.szTip[i] = c;
    }

    let exists = Shell_NotifyIconW(NIM_MODIFY, &nid);
    if !exists.as_bool() {
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        let _ = Shell_NotifyIconW(NIM_SETVERSION, &nid);
        nid.uFlags = NIF_TIP | NIF_SHOWTIP;
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

/// Runs the dedicated message pump thread for the system tray icon and context menu.
fn run_tray_thread(
    events: Arc<Mutex<VecDeque<TrayEvent>>>,
    active: Arc<AtomicBool>,
    window_hwnd_out: Arc<Mutex<Option<isize>>>,
    wakeup: Arc<dyn Fn() + Send + Sync>,
) {
    if let Ok(mut g) = GLOBAL_EVENTS.lock() {
        *g = Some(events);
    }
    if let Ok(mut g) = GLOBAL_WAKEUP.lock() {
        *g = Some(wakeup);
    }

    unsafe {
        unsafe extern "system" fn tray_wnd_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            let taskbar_created_msg = WM_TASKBAR_CREATED.load(Ordering::Relaxed);
            if taskbar_created_msg != 0 && msg == taskbar_created_msg {
                ensure_tray_icon_active(hwnd);
                return LRESULT(0);
            }

            match msg {
                WM_APP_TRAY_MSG => {
                    let mouse_event = (lparam.0 & 0xFFFF) as u32;
                    match mouse_event {
                        WM_LBUTTONUP | WM_LBUTTONDBLCLK | NIN_SELECT | NIN_KEYSELECT => {
                            dispatch_tray_event(TrayEvent::ShowWindow);
                            LRESULT(0)
                        }
                        WM_RBUTTONUP | WM_CONTEXTMENU => {
                            show_tray_context_menu(hwnd);
                            LRESULT(0)
                        }
                        _ => LRESULT(0),
                    }
                }
                WM_HOTKEY => {
                    let hotkey_id = wparam.0 as i32;
                    if hotkey_id == HOTKEY_ID_F9 {
                        dispatch_tray_event(TrayEvent::ToggleRecording);
                    } else if hotkey_id == HOTKEY_ID_F10 {
                        dispatch_tray_event(TrayEvent::TakeSnapshot);
                    }
                    LRESULT(0)
                }
                WM_TIMER => {
                    if wparam.0 == IDT_TRAY_WATCHDOG {
                        ensure_tray_icon_active(hwnd);
                    }
                    LRESULT(0)
                }
                WM_COMMAND => {
                    let cmd_id = wparam.0 & 0xFFFF;
                    handle_menu_command(cmd_id);
                    LRESULT(0)
                }
                WM_CLOSE => {
                    let _ = KillTimer(Some(hwnd), IDT_TRAY_WATCHDOG);
                    let _ = DestroyWindow(hwnd);
                    LRESULT(0)
                }
                WM_DESTROY => {
                    let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID_F9);
                    let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID_F10);
                    let _ = KillTimer(Some(hwnd), IDT_TRAY_WATCHDOG);
                    PostQuitMessage(0);
                    LRESULT(0)
                }
                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }

        let class_name = w!("ScreenRecorderTrayWindowClass");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(tray_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: windows::Win32::Foundation::HMODULE::default().into(),
            hIcon: HICON::default(),
            hCursor: windows::Win32::UI::WindowsAndMessaging::HCURSOR::default(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: class_name,
            hIconSm: HICON::default(),
        };

        RegisterClassExW(&wc);

        let hwnd = match CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name,
            w!("ScreenRecorderTrayMessageWindow"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            None,
            None,
        ) {
            Ok(h) => h,
            Err(_) => return,
        };

        if let Ok(mut guard) = window_hwnd_out.lock() {
            *guard = Some(hwnd.0 as isize);
        }

        // Register global Windows Hotkeys for F9 (Record) and F10 (Snapshot)
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_ID_F9, HOT_KEY_MODIFIERS(0), VK_F9.0 as u32);
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_ID_F10, HOT_KEY_MODIFIERS(0), VK_F10.0 as u32);

        let taskbar_msg = RegisterWindowMessageW(w!("TaskbarCreated"));
        WM_TASKBAR_CREATED.store(taskbar_msg, Ordering::SeqCst);

        let hicon = create_custom_tray_icon();
        TRAY_ICON_HANDLE.store(hicon.0 as isize, Ordering::SeqCst);

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 100,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
            uCallbackMessage: WM_APP_TRAY_MSG,
            hIcon: hicon,
            szTip: [0u16; 128],
            ..Default::default()
        };

        let tooltip = "Screen Recorder Pro";
        for (i, c) in tooltip.encode_utf16().enumerate().take(127) {
            nid.szTip[i] = c;
        }

        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        let _ = Shell_NotifyIconW(NIM_SETVERSION, &nid);

        nid.uFlags = NIF_TIP | NIF_SHOWTIP;
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;

        let _ = SetTimer(Some(hwnd), IDT_TRAY_WATCHDOG, 3000, None);

        let mut msg = MSG::default();
        while active.load(Ordering::Relaxed)
            && GetMessageW(&mut msg, None, 0, 0).as_bool()
        {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = KillTimer(Some(hwnd), IDT_TRAY_WATCHDOG);
        TRAY_ICON_HANDLE.store(0, Ordering::SeqCst);
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        if !hicon.0.is_null() {
            let _ = DestroyIcon(hicon);
        }
    }
}

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn show_tray_context_menu(hwnd: HWND) {
    let hmenu = match CreatePopupMenu() {
        Ok(m) => m,
        Err(_) => return,
    };

    let open_w = to_wide_null("Open Screen Recorder");
    let _ = AppendMenuW(hmenu, MF_STRING, IDM_OPEN, PCWSTR(open_w.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());

    let rec_w = to_wide_null("Start / Stop Recording [F9]");
    let _ = AppendMenuW(hmenu, MF_STRING, IDM_TOGGLE_RECORDING, PCWSTR(rec_w.as_ptr()));

    let snap_w = to_wide_null("Take Screenshot [F10]");
    let _ = AppendMenuW(hmenu, MF_STRING, IDM_TAKE_SNAPSHOT, PCWSTR(snap_w.as_ptr()));

    let folder_w = to_wide_null("Open Recordings Folder");
    let _ = AppendMenuW(hmenu, MF_STRING, IDM_OPEN_OUTPUT_DIR, PCWSTR(folder_w.as_ptr()));

    let replay_w = to_wide_null("Save Instant Replay (Last 30s)");
    let _ = AppendMenuW(hmenu, MF_STRING, IDM_INSTANT_REPLAY, PCWSTR(replay_w.as_ptr()));

    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());

    let exit_w = to_wide_null("Exit");
    let _ = AppendMenuW(hmenu, MF_STRING, IDM_EXIT, PCWSTR(exit_w.as_ptr()));

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);

    let _ = SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        hmenu,
        TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RETURNCMD,
        pt.x,
        pt.y,
        None,
        hwnd,
        None,
    );
    let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    let _ = DestroyMenu(hmenu);

    let cmd_id = cmd.0 as usize;
    if cmd_id > 0 {
        handle_menu_command(cmd_id);
    }
}

