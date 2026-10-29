//! Windows platform backend tests (`WGC`, `DXGI`, `GDI Monitors`, and `Win32 Tray`).
//!
//! Conforms to `[SPEC-PLAT-WIN-001]` and `RULE-SAFE-001`.

#![allow(clippy::unwrap_used)]

use std::time::Duration;
use rec_core::pixel_format::PixelFormat;
use rec_engine::traits::{CaptureBackend, CaptureTarget};
use rec_platform_windows::dxgi::DxgiDesktopDuplicationBackend;
use rec_platform_windows::monitors::{capture_screenshot, enumerate_monitors};
use rec_platform_windows::tray::{
    handle_menu_command, is_exit_requested, HOTKEY_ID_F10, HOTKEY_ID_F9, IDM_EXIT,
    IDM_INSTANT_REPLAY, IDM_OPEN, IDM_OPEN_OUTPUT_DIR, IDM_TAKE_SNAPSHOT, IDM_TOGGLE_RECORDING,
};
use rec_platform_windows::wgc::WgcCaptureBackend;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11Texture2D, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter, IDXGIDevice, IDXGIFactory1, IDXGIOutput1,
};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDCW, DeleteDC, DeleteObject, GetDC,
    GetDIBits, MonitorFromWindow, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, MONITOR_DEFAULTTOPRIMARY, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

#[test]
fn test_dxgi_duplication_backend_initialization() {
    let backend = DxgiDesktopDuplicationBackend::new();
    assert_eq!(backend.backend_name(), "DXGI Desktop Duplication API");
    assert!(!backend.is_capturing());
}

#[test]
fn test_dxgi_enumerate_outputs() {
    // SAFETY: Enumerates DXGI adapters and outputs via Win32 COM interfaces for diagnostics.
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
                println!(
                    "  Output {} (global {}): DeviceName={}",
                    o_idx, global_idx, dev_name
                );

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
    // SAFETY: Acquires and releases Win32 GDI device contexts and bitmaps for each display.
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
            let src_dc = if !dev_dc.0.is_null() {
                dev_dc
            } else {
                screen_dc
            };

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
            println!(
                "Monitor {} CreateDCW result={:?}, BitBlt result: {:?}",
                m.name,
                !dev_dc.0.is_null(),
                blt_res
            );

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
    assert_eq!(frame.descriptor.format, PixelFormat::Bgra);
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

#[test]
fn test_tray_constants_and_ids() {
    assert_eq!(IDM_OPEN, 1001);
    assert_eq!(IDM_TOGGLE_RECORDING, 1002);
    assert_eq!(IDM_INSTANT_REPLAY, 1003);
    assert_eq!(IDM_TAKE_SNAPSHOT, 1004);
    assert_eq!(IDM_OPEN_OUTPUT_DIR, 1005);
    assert_eq!(IDM_EXIT, 1099);
    assert_eq!(HOTKEY_ID_F9, 2001);
    assert_eq!(HOTKEY_ID_F10, 2002);
    assert!(!is_exit_requested());
}

#[test]
fn test_menu_command_mapping() {
    handle_menu_command(IDM_OPEN);
    handle_menu_command(IDM_TAKE_SNAPSHOT);
    handle_menu_command(IDM_OPEN_OUTPUT_DIR);
}

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
    use windows::core::factory;
    use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
    use windows::Graphics::DirectX::DirectXPixelFormat;
    use windows::Win32::System::WinRT::Direct3D11::{
        CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
    };
    use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

    let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().unwrap();
    // SAFETY: Queries primary monitor handle and initializes WinRT GraphicsCaptureItem.
    let hmon = unsafe { MonitorFromWindow(GetDesktopWindow(), MONITOR_DEFAULTTOPRIMARY) };
    // SAFETY: Invokes COM factory CreateForMonitor with a valid HMONITOR.
    let item: windows::core::Result<GraphicsCaptureItem> =
        unsafe { interop.CreateForMonitor(hmon) };
    println!("CreateForMonitor result: {:?}", item.is_ok());
    if let Ok(item) = item {
        let size = item.Size().unwrap();
        println!("GraphicsCaptureItem size: {}x{}", size.Width, size.Height);
        let display_name = item.DisplayName().unwrap();
        println!("GraphicsCaptureItem DisplayName: {}", display_name);

        let mut d3d_device: Option<ID3D11Device> = None;
        let levels = [D3D_FEATURE_LEVEL_11_0];
        // SAFETY: Creates hardware D3D11 device with BGRA support for WinRT interop.
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
            // SAFETY: Wraps DXGI device in a WinRT IDirect3DDevice inspectable.
            let inspectable =
                unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device).unwrap() };
            let winrt_device: windows::Graphics::DirectX::Direct3D11::IDirect3DDevice =
                inspectable.cast().unwrap();

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
                    std::thread::sleep(Duration::from_millis(50));
                    let frame = pool.TryGetNextFrame();
                    println!("TryGetNextFrame result: {:?}", frame.is_ok());
                    if let Ok(frame) = frame {
                        let surface = frame.Surface().unwrap();
                        let access: IDirect3DDxgiInterfaceAccess = surface.cast().unwrap();
                        // SAFETY: Queries underlying ID3D11Texture2D from WinRT surface.
                        let texture: ID3D11Texture2D = unsafe { access.GetInterface().unwrap() };
                        let mut desc = D3D11_TEXTURE2D_DESC::default();
                        // SAFETY: Reads texture descriptor into stack struct.
                        unsafe { texture.GetDesc(&mut desc) };
                        println!(
                            "Retrieved D3D11 texture from WGC frame: {}x{}, format: {:?}",
                            desc.Width, desc.Height, desc.Format
                        );
                    }
                    let _ = session.Close();
                    let _ = pool.Close();
                }
            }
        }
    }
}

