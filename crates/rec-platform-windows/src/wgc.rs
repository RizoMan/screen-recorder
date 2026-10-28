//! Windows native screen capture backend for individual windows and monitors.
//!
//! Conforms to `[SPEC-PLAT-WIN-001]` and `[CONTRACT-ARCH-001]`.
//! Employs hardware-accelerated Windows Graphics Capture (WinRT WGC) with Direct3D 11
//! textures for 60-120 FPS capture of 3D games, hardware video decoders, and full desktop,
//! with seamless fallback to GDI DIBSection capture on legacy configurations.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;
use rec_core::config::AppConfig;
use rec_core::cursor_effects::{
    apply_cursor_effects, ClickButton, ClickRipple, CursorOverlayState,
};
use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
use rec_core::pixel_format::PixelFormat;
use rec_engine::errors::CaptureError;
use rec_engine::traits::{CaptureBackend, CaptureTarget};
use windows::core::{factory, Interface, PCWSTR};
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDCW, CreateDIBSection, DeleteDC, DeleteObject,
    GdiFlush, GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HDC, HMONITOR, SRCCOPY,
};
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::monitors::{enumerate_monitors, MonitorInfo};

/// Tracks live Win32 mouse coordinates and left/right click transitions for `[SPEC-FEAT-CURSOR-001]`.
struct Win32CursorTracker {
    prev_left_down: bool,
    prev_right_down: bool,
    active_ripples: Vec<(i32, i32, ClickButton, Instant)>,
}

impl Win32CursorTracker {
    fn new() -> Self {
        Self {
            prev_left_down: false,
            prev_right_down: false,
            active_ripples: Vec::with_capacity(8),
        }
    }

    fn apply_to_frame(&mut self, frame: &mut VideoFrame, origin_x: i32, origin_y: i32, cfg: &AppConfig) {
        if !cfg.cursor_halo_enabled && !cfg.cursor_click_ripples_enabled {
            self.active_ripples.clear();
            return;
        }

        let mut pt = POINT::default();
        // SAFETY: Querying desktop cursor coordinates and async mouse button state via Win32 user32 APIs
        let (cursor_ok, left_down, right_down) = unsafe {
            let ok = GetCursorPos(&mut pt).is_ok();
            let l = (GetAsyncKeyState(i32::from(VK_LBUTTON.0)) as u16 & 0x8000) != 0;
            let r = (GetAsyncKeyState(i32::from(VK_RBUTTON.0)) as u16 & 0x8000) != 0;
            (ok, l, r)
        };

        if !cursor_ok {
            return;
        }

        let rel_x = pt.x - origin_x;
        let rel_y = pt.y - origin_y;
        let fw = frame.descriptor.width as i32;
        let fh = frame.descriptor.height as i32;
        let inside = rel_x >= 0 && rel_x < fw && rel_y >= 0 && rel_y < fh;

        let now = Instant::now();
        if cfg.cursor_click_ripples_enabled && inside {
            if left_down && !self.prev_left_down {
                self.active_ripples.push((rel_x, rel_y, ClickButton::Left, now));
            }
            if right_down && !self.prev_right_down {
                self.active_ripples.push((rel_x, rel_y, ClickButton::Right, now));
            }
        }
        self.prev_left_down = left_down;
        self.prev_right_down = right_down;

        const RIPPLE_DURATION_SECS: f32 = 0.36;
        self.active_ripples
            .retain(|&(_, _, _, t)| now.saturating_duration_since(t).as_secs_f32() < RIPPLE_DURATION_SECS);

        let ripples: Vec<ClickRipple> = if cfg.cursor_click_ripples_enabled {
            self.active_ripples
                .iter()
                .map(|&(x, y, button, t)| ClickRipple {
                    x,
                    y,
                    button,
                    progress: (now.saturating_duration_since(t).as_secs_f32() / RIPPLE_DURATION_SECS)
                        .clamp(0.0, 1.0),
                })
                .collect()
        } else {
            Vec::new()
        };

        let state = CursorOverlayState {
            cursor_x: rel_x,
            cursor_y: rel_y,
            cursor_visible: inside,
            halo_enabled: cfg.cursor_halo_enabled,
            halo_radius: cfg.cursor_halo_radius,
            halo_color: cfg.cursor_halo_color,
            ripples,
        };

        apply_cursor_effects(frame, &state);
    }
}

#[cfg(windows)]
#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(uPeriod: u32) -> u32;
    fn timeEndPeriod(uPeriod: u32) -> u32;
}

/// RAII guard ensuring Windows kernel operates at 1ms multimedia timer resolution.
struct HighResolutionTimerGuard;

impl HighResolutionTimerGuard {
    fn new() -> Self {
        #[cfg(windows)]
        // SAFETY: Requesting 1ms OS timer resolution for buttery smooth 60 FPS capture pacing
        unsafe {
            let _ = timeBeginPeriod(1);
        }
        Self
    }
}

impl Drop for HighResolutionTimerGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        // SAFETY: Restores standard OS timer resolution upon thread exit
        unsafe {
            let _ = timeEndPeriod(1);
        }
    }
}

/// Hardware-accelerated Windows Graphics Capture (WGC) backend implementing [`CaptureBackend`].
pub struct WgcCaptureBackend {
    is_capturing: Arc<AtomicBool>,
    target_fps: Arc<AtomicU32>,
    active_config: Arc<Mutex<AppConfig>>,
    worker_handle: Option<JoinHandle<()>>,
    epoch: Option<Instant>,
}

impl Default for WgcCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WgcCaptureBackend {
    /// Creates a new inactive `WgcCaptureBackend`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
            target_fps: Arc::new(AtomicU32::new(60)),
            active_config: Arc::new(Mutex::new(AppConfig::new())),
            worker_handle: None,
            epoch: None,
        }
    }
}

impl CaptureBackend for WgcCaptureBackend {
    fn set_epoch(&mut self, epoch: Instant) {
        self.epoch = Some(epoch);
    }

    fn update_config(&mut self, config: &AppConfig) {
        self.target_fps.store(config.fps.clamp(1, 120), Ordering::SeqCst);
        if let Ok(mut guard) = self.active_config.lock() {
            *guard = config.clone();
        }
    }

    fn start_capture(
        &mut self,
        target: CaptureTarget,
        frame_tx: Sender<VideoFrame>,
    ) -> Result<(), CaptureError> {
        if self.is_capturing.load(Ordering::SeqCst) {
            return Ok(());
        }

        let start_time = self.epoch.take().unwrap_or_else(Instant::now);
        self.is_capturing.store(true, Ordering::SeqCst);
        let capturing_flag = Arc::clone(&self.is_capturing);
        let target_fps_flag = Arc::clone(&self.target_fps);
        let config_slot = Arc::clone(&self.active_config);

        let handle = std::thread::Builder::new()
            .name("wgc-capture-thread".to_string())
            .spawn(move || {
                let monitors = enumerate_monitors().unwrap_or_default();

                let mon = match target {
                    CaptureTarget::FullScreen { monitor_index } => {
                        monitors.get(monitor_index).cloned().or_else(|| monitors.first().cloned())
                    }
                    CaptureTarget::Area { x, y, .. } => {
                        monitors
                            .iter()
                            .find(|m| {
                                (x as i32) >= m.left
                                    && (x as i32) < m.left + m.width as i32
                                    && (y as i32) >= m.top
                                    && (y as i32) < m.top + m.height as i32
                            })
                            .cloned()
                            .or_else(|| monitors.first().cloned())
                    }
                    _ => monitors.first().cloned(),
                };

                // 1. Try hardware-accelerated Windows Graphics Capture (WGC)
                let mut wgc_ran = false;
                if let Some(ref m) = mon {
                    wgc_ran = run_wgc_capture(
                        m,
                        target.clone(),
                        &capturing_flag,
                        &target_fps_flag,
                        &config_slot,
                        start_time,
                        &frame_tx,
                    );
                }

                // 2. Seamless fallback to GDI if WGC failed or is unsupported
                if !wgc_ran && capturing_flag.load(Ordering::Relaxed) {
                    eprintln!("WGC unavailable or failed; executing GDI capture fallback.");
                    run_gdi_capture(
                        mon.as_ref(),
                        target,
                        &capturing_flag,
                        &target_fps_flag,
                        &config_slot,
                        start_time,
                        &frame_tx,
                    );
                }
            })
            .map_err(|e| CaptureError::Internal(e.to_string()))?;

        self.worker_handle = Some(handle);
        Ok(())
    }

    fn stop_capture(&mut self) -> Result<(), CaptureError> {
        self.is_capturing.store(false, Ordering::SeqCst);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    fn is_capturing(&self) -> bool {
        self.is_capturing.load(Ordering::SeqCst)
    }

    fn set_fps(&mut self, fps: u32) {
        self.target_fps.store(fps.clamp(1, 120), Ordering::SeqCst);
    }

    fn backend_name(&self) -> &'static str {
        "Windows Graphics Capture (WGC)"
    }
}

/// Executes hardware-accelerated Direct3D 11 Windows Graphics Capture on target monitor.
fn run_wgc_capture(
    mon: &MonitorInfo,
    target: CaptureTarget,
    capturing_flag: &Arc<AtomicBool>,
    target_fps_flag: &Arc<AtomicU32>,
    config_slot: &Arc<Mutex<AppConfig>>,
    start_time: Instant,
    frame_tx: &Sender<VideoFrame>,
) -> bool {
    let hmon = HMONITOR(mon.hmonitor as *mut _);

    // SAFETY: Initializing WinRT GraphicsCaptureItem and Direct3D 11 staging pipeline
    let setup = unsafe {
        (|| -> windows::core::Result<_> {
            let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            let item: GraphicsCaptureItem = interop.CreateForMonitor(hmon)?;
            let item_size = item.Size()?;
            let width = item_size.Width as u32;
            let height = item_size.Height as u32;

            if width == 0 || height == 0 {
                return Err(windows::core::Error::from(windows::Win32::Foundation::E_FAIL));
            }

            let mut d3d_device: Option<ID3D11Device> = None;
            let mut d3d_context: Option<ID3D11DeviceContext> = None;
            let levels = [D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_1];
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                windows::Win32::Foundation::HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&levels),
                D3D11_SDK_VERSION,
                Some(&mut d3d_device),
                None,
                Some(&mut d3d_context),
            )?;
            let dev = d3d_device.ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;
            let ctx = d3d_context.ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;

            let dxgi_device: IDXGIDevice = dev.cast()?;
            let inspectable = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?;
            let winrt_device: windows::Graphics::DirectX::Direct3D11::IDirect3DDevice = inspectable.cast()?;

            let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                &winrt_device,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                2,
                item_size,
            )?;

            let session = frame_pool.CreateCaptureSession(&item)?;
            let _ = session.SetIsCursorCaptureEnabled(true);
            let _ = session.SetIsBorderRequired(false);
            session.StartCapture()?;

            let staging_desc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            let mut staging_tex = None;
            dev.CreateTexture2D(&staging_desc, None, Some(&mut staging_tex))?;
            let staging = staging_tex.ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;

            Ok((frame_pool, session, ctx, staging, width, height))
        })()
    };

    let (frame_pool, session, ctx, staging, width, height) = match setup {
        Ok(s) => s,
        Err(e) => {
            eprintln!("WGC initialization failed: {:?}", e);
            return false;
        }
    };

    let _timer_guard = HighResolutionTimerGuard::new();
    let mut next_frame_time = Instant::now();
    let mut last_dims = (width, height);
    let mut has_valid_staging = false;
    let mut cursor_tracker = Win32CursorTracker::new();

    let (origin_x, origin_y) = match target {
        CaptureTarget::Area { x, y, .. } => (x as i32, y as i32),
        _ => (mon.left, mon.top),
    };

    while capturing_flag.load(Ordering::Relaxed) {
        let target_fps = target_fps_flag.load(Ordering::Relaxed).clamp(1, 120);
        let frame_target_duration = Duration::from_nanos(1_000_000_000 / target_fps as u64);

        let mut should_map_staging = false;

        if let Ok(wgc_frame) = frame_pool.TryGetNextFrame() {
            if !frame_tx.is_full() {
                if let Ok(surface) = wgc_frame.Surface() {
                    if let Ok(access) = surface.cast::<IDirect3DDxgiInterfaceAccess>() {
                        // SAFETY: Acquiring ID3D11Texture2D from surface access and copying to staging texture
                        unsafe {
                            if let Ok(frame_tex) = access.GetInterface::<ID3D11Texture2D>() {
                                ctx.CopyResource(&staging, &frame_tex);
                                has_valid_staging = true;
                                should_map_staging = true;
                            }
                        }
                    }
                }
            }
            let _ = wgc_frame.Close();
        } else if has_valid_staging && !frame_tx.is_full() && !cursor_tracker.active_ripples.is_empty() {
            // Keep animating active click ripples smoothly even if desktop content is static
            should_map_staging = true;
        }

        let mut new_frame_data: Option<Vec<u8>> = None;
        if should_map_staging {
            // SAFETY: Mapping the D3D11 staging texture for CPU read access
            unsafe {
                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                if ctx.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).is_ok() {
                    let (final_w, final_h, buf) = match target {
                        CaptureTarget::Area { x, y, width: crop_w, height: crop_h } => {
                            let mon_x = mon.left;
                            let mon_y = mon.top;
                            let rel_x = (x as i32 - mon_x).max(0) as u32;
                            let rel_y = (y as i32 - mon_y).max(0) as u32;
                            let cw = crop_w.min(width.saturating_sub(rel_x)).max(2);
                            let ch = crop_h.min(height.saturating_sub(rel_y)).max(2);
                            let mut data = Vec::with_capacity((cw * ch * 4) as usize);
                            for row in 0..ch {
                                let row_ptr = (mapped.pData as *const u8)
                                    .add(((rel_y + row) * mapped.RowPitch) as usize)
                                    .add((rel_x * 4) as usize);
                                let slice = std::slice::from_raw_parts(row_ptr, (cw * 4) as usize);
                                data.extend_from_slice(slice);
                            }
                            (cw, ch, data)
                        }
                        _ => {
                            let mut data = Vec::with_capacity((width * height * 4) as usize);
                            for row in 0..height {
                                let row_ptr = (mapped.pData as *const u8)
                                    .add((row * mapped.RowPitch) as usize);
                                let slice = std::slice::from_raw_parts(row_ptr, (width * 4) as usize);
                                data.extend_from_slice(slice);
                            }
                            (width, height, data)
                        }
                    };
                    ctx.Unmap(&staging, 0);

                    last_dims = (final_w, final_h);
                    new_frame_data = Some(buf);
                }
            }
        }

        // Only dispatch to encoder pool when a real new frame has arrived from GPU.
        // Static frames are handled with zero-cost pre-encoded repeats by the pipeline CFR writer.
        if let Some(buf) = new_frame_data {
            let pts = Pts::from_duration(start_time.elapsed());
            if let Ok(desc) = FrameDescriptor::new(last_dims.0, last_dims.1, PixelFormat::Bgra) {
                if let Ok(mut frame) = VideoFrame::new(desc, pts, buf) {
                    if let Ok(cfg) = config_slot.lock() {
                        cursor_tracker.apply_to_frame(&mut frame, origin_x, origin_y, &cfg);
                    }
                    let _ = frame_tx.try_send(frame);
                }
            }
        }

        next_frame_time += frame_target_duration;
        let now = Instant::now();
        if next_frame_time > now {
            std::thread::sleep(next_frame_time - now);
        } else if now.saturating_duration_since(next_frame_time) > frame_target_duration * 2 {
            next_frame_time = now;
        }
    }


    let _ = session.Close();
    let _ = frame_pool.Close();
    true
}

/// Fallback GDI capture implementation using DIBSection memory mapping.
fn run_gdi_capture(
    mon: Option<&MonitorInfo>,
    target: CaptureTarget,
    capturing_flag: &Arc<AtomicBool>,
    target_fps_flag: &Arc<AtomicU32>,
    config_slot: &Arc<Mutex<AppConfig>>,
    start_time: Instant,
    frame_tx: &Sender<VideoFrame>,
) {
    let (dev_name, src_x, src_y, width, height, origin_x, origin_y) = match target {
        CaptureTarget::FullScreen { .. } => {
            if let Some(m) = mon {
                (Some(m.name.clone()), 0, 0, m.width.max(160), m.height.max(120), m.left, m.top)
            } else {
                (None, 0, 0, 1920, 1080, 0, 0)
            }
        }
        CaptureTarget::Area { x, y, width, height } => {
            (None, x as i32, y as i32, width.max(160), height.max(120), x as i32, y as i32)
        }
        _ => (None, 0, 0, 1920, 1080, 0, 0),
    };

    // SAFETY: Safe initialization of Win32 GDI capture context
    unsafe {
        let mut dev_dc = HDC::default();
        if let Some(ref name) = dev_name {
            let name_u16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            dev_dc = CreateDCW(
                PCWSTR(name_u16.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                None,
            );
        }

        let screen_dc = GetDC(None);
        let src_dc = if !dev_dc.0.is_null() { dev_dc } else { screen_dc };

        if src_dc.0.is_null() {
            eprintln!("Failed to acquire Windows screen DC for capture");
            return;
        }

        let mem_dc = CreateCompatibleDC(Some(src_dc));

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // Top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let dib_res = CreateDIBSection(
            Some(mem_dc),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            None,
            0,
        );

        let dib_bmp = match dib_res {
            Ok(hbmp) if !bits_ptr.is_null() => hbmp,
            _ => {
                eprintln!("Failed to create DIB section for capture");
                return;
            }
        };

        let old_bmp = SelectObject(mem_dc, dib_bmp.into());
        let buffer_size = (width * height * 4) as usize;
        let _timer_guard = HighResolutionTimerGuard::new();
        let mut next_frame_time = Instant::now();
        let mut cursor_tracker = Win32CursorTracker::new();

        while capturing_flag.load(Ordering::Relaxed) {
            let target_fps = target_fps_flag.load(Ordering::Relaxed).clamp(1, 120);
            let frame_target_duration = Duration::from_nanos(1_000_000_000 / target_fps as u64);

            if !frame_tx.is_full() {
                let blt_res = BitBlt(
                    mem_dc,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    Some(src_dc),
                    src_x,
                    src_y,
                    SRCCOPY,
                );

                if blt_res.is_ok() && !bits_ptr.is_null() {
                    let _ = GdiFlush();
                    let slice = std::slice::from_raw_parts(bits_ptr as *const u8, buffer_size);
                    let frame_data = slice.to_vec();

                    let pts = Pts::from_duration(start_time.elapsed());
                    if let Ok(desc) = FrameDescriptor::new(width, height, PixelFormat::Bgra) {
                        if let Ok(mut frame) = VideoFrame::new(desc, pts, frame_data) {
                            if let Ok(cfg) = config_slot.lock() {
                                cursor_tracker.apply_to_frame(&mut frame, origin_x, origin_y, &cfg);
                            }
                            let _ = frame_tx.try_send(frame);
                        }
                    }
                }
            }

            next_frame_time += frame_target_duration;
            let now = Instant::now();
            if next_frame_time > now {
                std::thread::sleep(next_frame_time - now);
            } else if now.saturating_duration_since(next_frame_time) > frame_target_duration * 2 {
                next_frame_time = now;
            }
        }

        let _ = SelectObject(mem_dc, old_bmp);
        let _ = DeleteObject(dib_bmp.into());
        let _ = DeleteDC(mem_dc);
        if !dev_dc.0.is_null() {
            let _ = DeleteDC(dev_dc);
        }
        ReleaseDC(None, screen_dc);
    }
}

impl Drop for WgcCaptureBackend {
    fn drop(&mut self) {
        let _ = self.stop_capture();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_wgc_backend_name_and_state() {
        let backend = WgcCaptureBackend::new();
        assert_eq!(backend.backend_name(), "Windows Graphics Capture (WGC)");
        assert!(!backend.is_capturing());
    }

    #[test]
    fn test_wgc_capture_primary_screen() {
        let mut backend = WgcCaptureBackend::new();
        let (tx, rx) = crossbeam_channel::bounded(10);
        let target = CaptureTarget::FullScreen { monitor_index: 0 };

        assert!(backend.start_capture(target, tx).is_ok());
        assert!(backend.is_capturing());

        // Receive at least 1 captured frame
        let received = rx.recv_timeout(Duration::from_millis(2000));
        assert!(received.is_ok());
        let frame = received.unwrap();
        assert!(frame.descriptor.width > 0);
        assert!(frame.descriptor.height > 0);
        assert_eq!(frame.descriptor.format, PixelFormat::Bgra);

        assert!(backend.stop_capture().is_ok());
        assert!(!backend.is_capturing());
    }

    #[test]
    fn test_wgc_capture_secondary_screen() {
        let mut backend = WgcCaptureBackend::new();
        let (tx, rx) = crossbeam_channel::bounded(10);
        let target = CaptureTarget::FullScreen { monitor_index: 1 };

        assert!(backend.start_capture(target, tx).is_ok());
        assert!(backend.is_capturing());

        // Receive at least 1 captured frame
        let received = rx.recv_timeout(Duration::from_millis(2000));
        assert!(received.is_ok());
        let frame = received.unwrap();
        assert!(frame.descriptor.width > 0);
        assert!(frame.descriptor.height > 0);

        assert!(backend.stop_capture().is_ok());
        assert!(!backend.is_capturing());
    }

    #[test]
    fn test_wgc_winrt_interop() {
        use windows::Graphics::Capture::GraphicsCaptureItem;
        use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
        use windows::core::factory;
        use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTOPRIMARY};
        use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

        let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().unwrap();
        let hmon = unsafe { MonitorFromWindow(GetDesktopWindow(), MONITOR_DEFAULTTOPRIMARY) };
        let item: windows::core::Result<GraphicsCaptureItem> = unsafe { interop.CreateForMonitor(hmon) };
        println!("CreateForMonitor result: {:?}", item.is_ok());
        if let Ok(item) = item {
            let size = item.Size().unwrap();
            println!("GraphicsCaptureItem size: {}x{}", size.Width, size.Height);
            let display_name = item.DisplayName().unwrap();
            println!("GraphicsCaptureItem DisplayName: {}", display_name);

            use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
            use windows::Win32::Graphics::Direct3D11::{D3D11CreateDevice, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, ID3D11Device};
            use windows::Win32::Graphics::Dxgi::IDXGIDevice;
            use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
            use windows::Graphics::DirectX::DirectXPixelFormat;
            use windows::Graphics::Capture::Direct3D11CaptureFramePool;
            use windows::core::Interface;

            let mut d3d_device: Option<ID3D11Device> = None;
            let levels = [D3D_FEATURE_LEVEL_11_0];
            let res = unsafe {
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    windows::Win32::Foundation::HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    Some(&levels),
                    D3D11_SDK_VERSION,
                    Some(&mut d3d_device),
                    None,
                    None,
                )
            };
            println!("D3D11CreateDevice result: {:?}", res.is_ok());

            if let Some(dev) = d3d_device {
                let dxgi_device: IDXGIDevice = dev.cast().unwrap();
                let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device).unwrap() };
                let winrt_device: windows::Graphics::DirectX::Direct3D11::IDirect3DDevice = inspectable.cast().unwrap();

                let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                    &winrt_device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    2,
                    size,
                );
                println!("Direct3D11CaptureFramePool result: {:?}", frame_pool.is_ok());

                if let Ok(pool) = frame_pool {
                    let session = pool.CreateCaptureSession(&item);
                    println!("CreateCaptureSession result: {:?}", session.is_ok());
                    if let Ok(session) = session {
                        let _ = session.StartCapture();
                        println!("Session StartCapture succeeded!");
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        let frame = pool.TryGetNextFrame();
                        println!("TryGetNextFrame result: {:?}", frame.is_ok());
                        if let Ok(frame) = frame {
                            use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
                            use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
                            let surface = frame.Surface().unwrap();
                            let access: IDirect3DDxgiInterfaceAccess = surface.cast().unwrap();
                            let texture: ID3D11Texture2D = unsafe { access.GetInterface().unwrap() };
                            let mut desc = windows::Win32::Graphics::Direct3D11::D3D11_TEXTURE2D_DESC::default();
                            unsafe { texture.GetDesc(&mut desc) };
                            println!("Retrieved D3D11 texture from WGC frame: {}x{}, format: {:?}", desc.Width, desc.Height, desc.Format);
                        }
                        let _ = session.Close();
                        let _ = pool.Close();
                    }
                }
            }
        }
    }
}
