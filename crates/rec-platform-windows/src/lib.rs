//! `rec-platform-windows`: Windows-native screen capture and audio backends.
//!
//! # Architecture Role
//!
//! Conforms to `[SPEC-PLAT-WIN-001]`. Implements the [`rec_engine::traits::CaptureBackend`]
//! and [`rec_engine::traits::AudioBackend`] interfaces for Windows using Windows Graphics
//! Capture (WGC / WinRT), DXGI Desktop Duplication API, and WASAPI audio loopback.

pub mod dxgi;
pub mod errors;
pub mod monitors;
pub mod wasapi;
pub mod wgc;

#[cfg(target_os = "windows")]
pub mod tray;

pub use dxgi::DxgiDesktopDuplicationBackend;
pub use errors::WindowsPlatformError;
pub use monitors::{capture_screenshot, enumerate_monitors, MonitorInfo};
pub use wasapi::WasapiAudioBackend;
pub use wgc::WgcCaptureBackend;

#[cfg(target_os = "windows")]
pub use tray::{is_exit_requested, request_app_exit, restore_main_window, TrayEvent, Win32TrayManager};

/// Attaches standard I/O streams to the parent terminal console if launched from a command line.
///
/// Has no effect if the process was launched directly from Windows Explorer, Start Menu,
/// or a desktop shortcut without a parent console.
pub fn attach_parent_console() {
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn AttachConsole(dw_process_id: u32) -> i32;
        }
        // SAFETY: Invoking Win32 AttachConsole with ATTACH_PARENT_PROCESS (0xFFFFFFFF)
        // simply attaches to the calling terminal's existing console buffer or returns 0
        // harmlessly if no parent console is attached.
        unsafe {
            AttachConsole(0xFFFF_FFFF);
        }
    }
}
