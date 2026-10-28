//! Display monitor and top-level window discovery for Windows.
//!
//! Conforms to `[SPEC-PLAT-WIN-001]`.

use rec_engine::errors::CaptureError;
use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    EnumDisplayMonitors, GetDC, GetDIBits, GetMonitorInfoW, ReleaseDC, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW, SRCCOPY,
};

const MONITORINFOF_PRIMARY: u32 = 1;

/// Information about an enumerated monitor display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorInfo {
    /// Monitor index.
    pub index: usize,
    /// Friendly system name of the monitor.
    pub name: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// True if this is the system primary monitor.
    pub is_primary: bool,
    /// X offset in virtual screen space.
    pub left: i32,
    /// Y offset in virtual screen space.
    pub top: i32,
    /// Native Win32 HMONITOR handle value.
    pub hmonitor: usize,
}

/// Enumerates all active display monitors connected to the system using Win32 APIs.
///
/// # Returns
///
/// A vector of [`MonitorInfo`] ordered with the primary display first.
///
/// # Errors
///
/// Returns [`CaptureError::TargetNotFound`] if no monitors are discovered.
pub fn enumerate_monitors() -> Result<Vec<MonitorInfo>, CaptureError> {
    unsafe extern "system" fn enum_proc(
        hmon: HMONITOR,
        _hdc: HDC,
        _lprect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let list = &mut *(lparam.0 as *mut Vec<MonitorInfo>);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

        if GetMonitorInfoW(hmon, &mut info as *mut _ as *mut MONITORINFO).as_bool() {
            let rect = info.monitorInfo.rcMonitor;
            let is_primary = (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0;
            let width = (rect.right - rect.left).max(1) as u32;
            let height = (rect.bottom - rect.top).max(1) as u32;
            let name = String::from_utf16_lossy(&info.szDevice)
                .trim_matches('\0')
                .to_string();

            list.push(MonitorInfo {
                index: 0, // Assigned after sorting
                name,
                width,
                height,
                is_primary,
                left: rect.left,
                top: rect.top,
                hmonitor: hmon.0 as usize,
            });
        }
        BOOL(1)
    }

    let mut monitors: Vec<MonitorInfo> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_proc),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }

    if monitors.is_empty() {
        return Err(CaptureError::TargetNotFound(
            "No active display monitors detected".to_string(),
        ));
    }

    // Sort so primary monitor is index 0, followed by secondary monitors
    monitors.sort_by(|a, b| b.is_primary.cmp(&a.is_primary));

    // Re-index sequentially
    for (i, m) in monitors.iter_mut().enumerate() {
        m.index = i;
    }


    Ok(monitors)
}

struct ScopedDc {
    hdc: HDC,
    hwnd: Option<windows::Win32::Foundation::HWND>,
}

impl Drop for ScopedDc {
    fn drop(&mut self) {
        if !self.hdc.0.is_null() {
            // SAFETY: ReleaseDC safely releases the DC associated with hwnd (or desktop DC if hwnd is None).
            unsafe {
                ReleaseDC(self.hwnd, self.hdc);
            }
        }
    }
}

struct ScopedMemDc(HDC);

impl Drop for ScopedMemDc {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            // SAFETY: DeleteDC frees compatible memory device contexts.
            unsafe {
                let _ = DeleteDC(self.0);
            }
        }
    }
}

struct ScopedBitmap(windows::Win32::Graphics::Gdi::HBITMAP);

impl Drop for ScopedBitmap {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            // SAFETY: DeleteObject frees the GDI bitmap resource.
            unsafe {
                let _ = DeleteObject(self.0.into());
            }
        }
    }
}

/// Captures a one-shot screenshot of the specified [`rec_engine::traits::CaptureTarget`].
///
/// Captures high-fidelity desktop or window pixels via native Win32 GDI into a [`rec_core::frame::VideoFrame`]
/// with [`rec_core::pixel_format::PixelFormat::Bgra`].
///
/// # Arguments
///
/// * `target` - Capture target specification (`FullScreen`, `Area`, or `Window`).
///
/// # Returns
///
/// A valid [`rec_core::frame::VideoFrame`] containing raw BGRA pixel data.
///
/// # Errors
///
/// Returns [`CaptureError::TargetNotFound`] if the monitor index or window does not exist,
/// or [`CaptureError::Internal`] if GDI operations fail.
pub fn capture_screenshot(
    target: &rec_engine::traits::CaptureTarget,
) -> Result<rec_core::frame::VideoFrame, CaptureError> {
    // Acquire desktop DC covering all displays
    // SAFETY: GetDC(None) retrieves the primary virtual desktop DC covering all monitors.
    let raw_screen_dc = unsafe { GetDC(None) };
    if raw_screen_dc.0.is_null() {
        return Err(CaptureError::Internal(
            "Failed to acquire desktop DC".to_string(),
        ));
    }
    let scoped_screen = ScopedDc {
        hdc: raw_screen_dc,
        hwnd: None,
    };

    match target {
        rec_engine::traits::CaptureTarget::FullScreen { monitor_index } => {
            let monitors = enumerate_monitors()?;
            let mon = monitors.get(*monitor_index).ok_or_else(|| {
                CaptureError::TargetNotFound(format!(
                    "Monitor index {} out of bounds (total: {})",
                    monitor_index,
                    monitors.len()
                ))
            })?;

            capture_dc_rect(scoped_screen.hdc, mon.left, mon.top, mon.width, mon.height)
        }
        rec_engine::traits::CaptureTarget::Area {
            x,
            y,
            width,
            height,
        } => {
            if *width == 0 || *height == 0 {
                return Err(CaptureError::TargetNotFound(
                    "Capture area width and height must be greater than zero".to_string(),
                ));
            }
            capture_dc_rect(scoped_screen.hdc, *x as i32, *y as i32, *width, *height)
        }
        rec_engine::traits::CaptureTarget::Window { window_id } => {
            let hwnd = windows::Win32::Foundation::HWND(*window_id as *mut std::ffi::c_void);
            // SAFETY: Checking validity of window handle.
            let is_valid = unsafe {
                windows::Win32::UI::WindowsAndMessaging::IsWindow(Some(hwnd)).as_bool()
            };
            if !is_valid {
                return Err(CaptureError::TargetNotFound(format!(
                    "Window handle {:#x} is invalid or closed",
                    window_id
                )));
            }

            let mut rect = RECT::default();
            // SAFETY: GetWindowRect called with valid HWND pointer.
            let rect_res = unsafe {
                windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect)
            };
            if rect_res.is_err() {
                return Err(CaptureError::Internal(
                    "Failed to retrieve window rectangle".to_string(),
                ));
            }

            let w = (rect.right - rect.left).max(1) as u32;
            let h = (rect.bottom - rect.top).max(1) as u32;

            capture_dc_rect(scoped_screen.hdc, rect.left, rect.top, w, h)
        }
    }
}

fn capture_dc_rect(
    src_dc: HDC,
    src_x: i32,
    src_y: i32,
    width: u32,
    height: u32,
) -> Result<rec_core::frame::VideoFrame, CaptureError> {
    // SAFETY: Creating memory device context compatible with src_dc.
    let mem_dc_raw = unsafe { CreateCompatibleDC(Some(src_dc)) };
    if mem_dc_raw.0.is_null() {
        return Err(CaptureError::Internal(
            "CreateCompatibleDC failed".to_string(),
        ));
    }
    let mem_dc = ScopedMemDc(mem_dc_raw);

    // SAFETY: Creating compatible bitmap.
    let bmp_raw = unsafe { CreateCompatibleBitmap(src_dc, width as i32, height as i32) };
    if bmp_raw.0.is_null() {
        return Err(CaptureError::Internal(
            "CreateCompatibleBitmap failed".to_string(),
        ));
    }
    let bmp = ScopedBitmap(bmp_raw);

    // SAFETY: Selecting bitmap into memory DC.
    let old_bmp = unsafe { SelectObject(mem_dc.0, bmp.0.into()) };

    // SAFETY: BitBlt performs the hardware/software blit from src_dc to mem_dc.
    let blt_res = unsafe {
        BitBlt(
            mem_dc.0,
            0,
            0,
            width as i32,
            height as i32,
            Some(src_dc),
            src_x,
            src_y,
            SRCCOPY,
        )
    };

    if let Err(ref e) = blt_res {
        log::debug!("BitBlt returned warning: {:?}; attempting GetDIBits", e);
    }

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // Negative height specifies top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let buffer_size = (width * height * 4) as usize;
    let mut raw_bytes = vec![0u8; buffer_size];

    // SAFETY: GetDIBits extracts 32-bit BGRA pixels into raw_bytes.
    let lines = unsafe {
        GetDIBits(
            mem_dc.0,
            bmp.0,
            0,
            height,
            Some(raw_bytes.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };

    // SAFETY: Restoring original bitmap object before dropping ScopedBitmap and ScopedMemDc.
    unsafe {
        SelectObject(mem_dc.0, old_bmp);
    }

    if lines != height as i32 {
        return Err(CaptureError::Internal(format!(
            "GetDIBits extracted {} lines instead of requested {}",
            lines, height
        )));
    }

    let descriptor = rec_core::frame::FrameDescriptor::new(
        width,
        height,
        rec_core::pixel_format::PixelFormat::Bgra,
    )
    .map_err(|e| CaptureError::Internal(format!("Invalid frame descriptor: {}", e)))?;

    rec_core::frame::VideoFrame::new(descriptor, rec_core::frame::Pts::from_nanos(0), raw_bytes)
        .map_err(|e| CaptureError::Internal(format!("Failed to create VideoFrame: {}", e)))
}


#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rec_engine::traits::CaptureTarget;
    use windows::Win32::Graphics::Gdi::CreateDCW;

    #[test]
    fn test_enumerate_monitors() {
        let res = enumerate_monitors();
        assert!(res.is_ok());
        let monitors = res.unwrap();
        assert!(!monitors.is_empty());
        assert!(monitors[0].is_primary, "Primary monitor must be index 0");
        assert!(monitors[0].width > 0);
        assert!(monitors[0].height > 0);
        println!("Enumerated monitors: {:?}", monitors);
    }

    #[test]
    fn test_gdi_capture_each_monitor() {
        let monitors = enumerate_monitors().unwrap();
        unsafe {
            let screen_dc = GetDC(None);
            assert!(!screen_dc.0.is_null());

            for m in &monitors {
                let name_utf16: Vec<u16> = m.name.encode_utf16().chain(std::iter::once(0)).collect();
                let dev_dc = CreateDCW(
                    windows::core::PCWSTR(name_utf16.as_ptr()),
                    windows::core::PCWSTR::null(),
                    windows::core::PCWSTR::null(),
                    None,
                );
                let src_dc = if !dev_dc.0.is_null() { dev_dc } else { screen_dc };

                let mem_dc = CreateCompatibleDC(Some(src_dc));
                assert!(!mem_dc.0.is_null());
                let bmp = CreateCompatibleBitmap(src_dc, m.width as i32, m.height as i32);
                assert!(!bmp.0.is_null());
                let old_bmp = SelectObject(mem_dc, bmp.into());

                let blt_res = BitBlt(
                    mem_dc,
                    0,
                    0,
                    m.width as i32,
                    m.height as i32,
                    Some(src_dc),
                    0,
                    0,
                    SRCCOPY,
                );
                println!("Monitor {} CreateDCW result={:?}, BitBlt result: {:?}", m.name, !dev_dc.0.is_null(), blt_res);

                let mut bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: m.width as i32,
                        biHeight: -(m.height as i32), // Top-down
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                let mut raw_bytes = vec![0u8; (m.width * m.height * 4) as usize];
                let lines = GetDIBits(
                    mem_dc,
                    bmp,
                    0,
                    m.height,
                    Some(raw_bytes.as_mut_ptr() as *mut _),
                    &mut bmi,
                    DIB_RGB_COLORS,
                );
                assert_eq!(lines, m.height as i32);
                println!(
                    "Monitor {}: successfully captured {}x{} (bytes: {})",
                    m.name,
                    m.width,
                    m.height,
                    raw_bytes.len()
                );

                let _ = SelectObject(mem_dc, old_bmp);
                let _ = DeleteObject(bmp.into());
                let _ = DeleteDC(mem_dc);
            }

            ReleaseDC(None, screen_dc);
        }
    }

    #[test]
    fn test_capture_screenshot_fullscreen_and_area() {
        let fullscreen_target = CaptureTarget::FullScreen { monitor_index: 0 };
        let res = capture_screenshot(&fullscreen_target);
        if let Err(ref e) = res {
            eprintln!("FullScreen error: {:?}", e);
        }
        assert!(res.is_ok(), "FullScreen screenshot capture should succeed");
        let frame = res.unwrap();
        assert!(frame.descriptor.width > 0);
        assert!(frame.descriptor.height > 0);
        assert_eq!(frame.descriptor.format, rec_core::pixel_format::PixelFormat::Bgra);
        assert_eq!(frame.data.len(), frame.descriptor.required_buffer_size());

        let area_target = CaptureTarget::Area {
            x: 0,
            y: 0,
            width: 320,
            height: 240,
        };
        let area_res = capture_screenshot(&area_target);
        if let Err(ref e) = area_res {
            eprintln!("Area screenshot error: {:?}", e);
        }
        assert!(area_res.is_ok(), "Area screenshot capture should succeed");
        let area_frame = area_res.unwrap();
        assert_eq!(area_frame.descriptor.width, 320);
        assert_eq!(area_frame.descriptor.height, 240);
        assert_eq!(area_frame.data.len(), 320 * 240 * 4);
    }
}

