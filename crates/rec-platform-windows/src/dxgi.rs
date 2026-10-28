//! DXGI Desktop Duplication API capture backend for ultra-low latency desktop streaming.
//!
//! Conforms to `[SPEC-PLAT-WIN-001]` and `[CONTRACT-ARCH-001]`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;
use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
use rec_core::pixel_format::PixelFormat;
use rec_engine::errors::CaptureError;
use rec_engine::traits::{CaptureBackend, CaptureTarget};

use windows::core::Interface;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice,
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE,
    D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter, IDXGIFactory1, IDXGIOutput1,
    DXGI_OUTDUPL_FRAME_INFO,
};

/// DXGI Desktop Duplication backend implementing [`CaptureBackend`].
pub struct DxgiDesktopDuplicationBackend {
    is_capturing: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
    epoch: Option<Instant>,
}

impl Default for DxgiDesktopDuplicationBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DxgiDesktopDuplicationBackend {
    /// Creates a new inactive `DxgiDesktopDuplicationBackend`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
            worker_handle: None,
            epoch: None,
        }
    }
}

impl CaptureBackend for DxgiDesktopDuplicationBackend {
    fn set_epoch(&mut self, epoch: Instant) {
        self.epoch = Some(epoch);
    }

    fn start_capture(
        &mut self,
        target: CaptureTarget,
        frame_tx: Sender<VideoFrame>,
    ) -> Result<(), CaptureError> {
        if self.is_capturing.load(Ordering::SeqCst) {
            return Ok(());
        }

        let monitor_index = match target {
            CaptureTarget::FullScreen { monitor_index } => monitor_index,
            _ => 0,
        };

        let start_time = self.epoch.take().unwrap_or_else(Instant::now);
        self.is_capturing.store(true, Ordering::SeqCst);
        let capturing_flag = Arc::clone(&self.is_capturing);

        let handle = std::thread::Builder::new()
            .name("dxgi-duplication-thread".to_string())
            .spawn(move || {

                // 1. Locate the adapter that owns the target monitor output and create D3D11 device on that adapter
                let (device, context, duplication) = unsafe {
                    let factory: IDXGIFactory1 = match CreateDXGIFactory1() {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Failed to create DXGIFactory1: {:?}", e);
                            return;
                        }
                    };

                    let mut found_setup = None;
                    let mut current_global_output_idx = 0usize;

                    for adapter_idx in 0.. {
                        let adapter = match factory.EnumAdapters1(adapter_idx) {
                            Ok(a) => a,
                            Err(_) => break,
                        };

                        for output_idx in 0.. {
                            let output = match adapter.EnumOutputs(output_idx) {
                                Ok(o) => o,
                                Err(_) => break,
                            };

                            if current_global_output_idx == monitor_index {
                                let mut dev = None;
                                let mut ctx = None;
                                let levels = [D3D_FEATURE_LEVEL_11_0];

                                if let Ok(adapter0) = adapter.cast::<IDXGIAdapter>() {
                                    let dev_res = D3D11CreateDevice(
                                        Some(&adapter0),
                                        D3D_DRIVER_TYPE_UNKNOWN,
                                        windows::Win32::Foundation::HMODULE::default(),
                                        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                                        Some(&levels),
                                        D3D11_SDK_VERSION,
                                        Some(&mut dev),
                                        None,
                                        Some(&mut ctx),
                                    );

                                    if dev_res.is_ok() {
                                        if let (Some(d), Some(c)) = (dev, ctx) {
                                            if let Ok(output1) = output.cast::<IDXGIOutput1>() {
                                                match output1.DuplicateOutput(&d) {
                                                    Ok(dup) => {
                                                        eprintln!(
                                                            "DuplicateOutput succeeded on adapter {} output {} (target monitor {})",
                                                            adapter_idx, output_idx, monitor_index
                                                        );
                                                        found_setup = Some((d, c, dup));
                                                    }
                                                    Err(e) => {
                                                        eprintln!(
                                                            "DuplicateOutput failed on adapter {} output {}: {:?}",
                                                            adapter_idx, output_idx, e
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                break;
                            }

                            current_global_output_idx += 1;
                        }

                        if found_setup.is_some() || current_global_output_idx > monitor_index {
                            break;
                        }
                    }

                    if found_setup.is_none() {
                        eprintln!("DXGI duplication not available; using WGC/Monitor capture fallback.");
                        let mut wgc = crate::wgc::WgcCaptureBackend::new();
                        let _ = wgc.start_capture(target, frame_tx);
                        while capturing_flag.load(Ordering::Relaxed) {
                            std::thread::sleep(Duration::from_millis(50));
                        }
                        let _ = wgc.stop_capture();
                        return;
                    }

                    match found_setup {
                        Some(setup) => setup,
                        None => return,
                    }
                };

                let mut staging_texture: Option<ID3D11Texture2D> = None;
                let mut texture_width = 0u32;
                let mut texture_height = 0u32;
                let mut captured_frames = 0u64;

                while capturing_flag.load(Ordering::Relaxed) {
                    let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
                    let mut desktop_resource = None;

                    let acq = unsafe {
                        duplication.AcquireNextFrame(50, &mut frame_info, &mut desktop_resource)
                    };

                    if acq.is_ok() {
                        if frame_tx.is_full() {
                            let _ = unsafe { duplication.ReleaseFrame() };
                            std::thread::sleep(Duration::from_millis(4));
                            continue;
                        }
                        if let Some(resource) = desktop_resource {
                            if let Ok(desktop_texture) = resource.cast::<ID3D11Texture2D>() {
                                let mut desc = D3D11_TEXTURE2D_DESC::default();
                                unsafe { desktop_texture.GetDesc(&mut desc) };

                                if staging_texture.is_none()
                                    || texture_width != desc.Width
                                    || texture_height != desc.Height
                                {
                                    desc.Usage = D3D11_USAGE_STAGING;
                                    desc.BindFlags = 0;
                                    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                                    desc.MiscFlags = 0;

                                    let mut tex: Option<ID3D11Texture2D> = None;
                                    if unsafe { device.CreateTexture2D(&desc, None, Some(&mut tex)) }.is_ok() {
                                        staging_texture = tex;
                                        texture_width = desc.Width;
                                        texture_height = desc.Height;
                                    }
                                }

                                if let Some(ref staging) = staging_texture {
                                    unsafe { context.CopyResource(staging, &desktop_texture) };
                                    let _ = unsafe { duplication.ReleaseFrame() };

                                    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                                    let map_res = unsafe {
                                        context.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                                    };

                                    if map_res.is_ok() {
                                        let (final_w, final_h, raw_rgba) = match target {
                                            CaptureTarget::Area { x, y, width, height } => {
                                                let crop_x = x.min(texture_width.saturating_sub(1));
                                                let crop_y = y.min(texture_height.saturating_sub(1));
                                                let crop_w = width.min(texture_width - crop_x).max(2);
                                                let crop_h = height.min(texture_height - crop_y).max(2);

                                                let mut cropped_bytes = Vec::with_capacity((crop_w * crop_h * 4) as usize);
                                                for row in 0..crop_h {
                                                    let row_src = unsafe {
                                                        (mapped.pData as *const u8)
                                                            .add(((crop_y + row) * mapped.RowPitch) as usize)
                                                            .add((crop_x * 4) as usize)
                                                    };
                                                    let slice = unsafe {
                                                        std::slice::from_raw_parts(row_src, (crop_w * 4) as usize)
                                                    };
                                                    for chunk in slice.chunks_exact(4) {
                                                        cropped_bytes.push(chunk[2]); // R
                                                        cropped_bytes.push(chunk[1]); // G
                                                        cropped_bytes.push(chunk[0]); // B
                                                        cropped_bytes.push(255);      // A
                                                    }
                                                }
                                                (crop_w, crop_h, cropped_bytes)
                                            }
                                            _ => {
                                                let mut bytes = Vec::with_capacity((texture_width * texture_height * 4) as usize);
                                                for row in 0..texture_height {
                                                    let row_src = unsafe {
                                                        (mapped.pData as *const u8).add((row * mapped.RowPitch) as usize)
                                                    };
                                                    let slice = unsafe {
                                                        std::slice::from_raw_parts(row_src, (texture_width * 4) as usize)
                                                    };
                                                    for chunk in slice.chunks_exact(4) {
                                                        bytes.push(chunk[2]); // R
                                                        bytes.push(chunk[1]); // G
                                                        bytes.push(chunk[0]); // B
                                                        bytes.push(255);      // A
                                                    }
                                                }
                                                (texture_width, texture_height, bytes)
                                            }
                                        };

                                        unsafe { context.Unmap(staging, 0) };

                                        let pts = Pts::from_duration(start_time.elapsed());
                                        if let Ok(desc) = FrameDescriptor::new(final_w, final_h, PixelFormat::Rgba) {
                                            if let Ok(frame) = VideoFrame::new(desc, pts, raw_rgba) {
                                                let _ = frame_tx.try_send(frame);
                                                captured_frames += 1;
                                                if captured_frames <= 3 || captured_frames.is_multiple_of(60) {
                                                    eprintln!("DXGI captured frame #{}: {}x{}", captured_frames, final_w, final_h);
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    let _ = unsafe { duplication.ReleaseFrame() };
                                }
                            } else {
                                let _ = unsafe { duplication.ReleaseFrame() };
                            }
                        } else {
                            let _ = unsafe { duplication.ReleaseFrame() };
                        }
                    } else {
                        // Wait briefly before next polling if no frame arrived
                        std::thread::sleep(Duration::from_millis(8));
                    }
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

    fn backend_name(&self) -> &'static str {
        "DXGI Desktop Duplication API"
    }
}

impl Drop for DxgiDesktopDuplicationBackend {
    fn drop(&mut self) {
        let _ = self.stop_capture();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dxgi_duplication_backend_initialization() {
        let backend = DxgiDesktopDuplicationBackend::new();
        assert_eq!(backend.backend_name(), "DXGI Desktop Duplication API");
        assert!(!backend.is_capturing());
    }

    #[test]
    fn test_dxgi_enumerate_outputs() {
        unsafe {
            let factory: IDXGIFactory1 = match CreateDXGIFactory1() {
                Ok(f) => f,
                Err(e) => {
                    println!("Failed CreateDXGIFactory1: {:?}", e);
                    return;
                }
            };
            let mut global_idx = 0;
            for a_idx in 0.. {
                let adapter = match factory.EnumAdapters1(a_idx) {
                    Ok(a) => a,
                    Err(_) => break,
                };
                let desc = adapter.GetDesc1().unwrap_or_default();
                let name = String::from_utf16_lossy(&desc.Description);
                println!("Adapter {}: {}", a_idx, name);

                for o_idx in 0.. {
                    let output = match adapter.EnumOutputs(o_idx) {
                        Ok(o) => o,
                        Err(_) => break,
                    };
                    let desc = output.GetDesc().unwrap_or_default();
                    let dev_name = String::from_utf16_lossy(&desc.DeviceName);
                    println!("  Output {} (global {}): DeviceName={}", o_idx, global_idx, dev_name);

                    // Test duplication
                    let mut dev = None;
                    let mut ctx = None;
                    let levels = [D3D_FEATURE_LEVEL_11_0];
                    if let Ok(a0) = adapter.cast::<IDXGIAdapter>() {
                        let _ = D3D11CreateDevice(
                            Some(&a0),
                            D3D_DRIVER_TYPE_UNKNOWN,
                            windows::Win32::Foundation::HMODULE::default(),
                            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                            Some(&levels),
                            D3D11_SDK_VERSION,
                            Some(&mut dev),
                            None,
                            Some(&mut ctx),
                        );
                        if let (Some(d), Some(_c)) = (dev, ctx) {
                            if let Ok(output1) = output.cast::<IDXGIOutput1>() {
                                match output1.DuplicateOutput(&d) {
                                    Ok(_dup) => println!("    -> DuplicateOutput SUCCESS!"),
                                    Err(e) => println!("    -> DuplicateOutput FAILED: {:?}", e),
                                }
                            }
                        }
                    }
                    global_idx += 1;
                }
            }
        }
    }
}

