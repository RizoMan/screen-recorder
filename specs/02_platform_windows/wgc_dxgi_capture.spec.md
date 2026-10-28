# SPECIFICATION: WINDOWS GRAPHICS CAPTURE (WGC), DXGI & WASAPI
Spec ID: `[SPEC-PLAT-WIN-001]`
Status: APPROVED
Domain: Windows Platform Implementation

---

## 1. Overview
Defines screen capture and audio recording mechanisms for Windows using Windows Graphics Capture (WGC / WinRT) for windows/displays, DXGI Desktop Duplication API for ultra-low latency full-screen gaming capture, and WASAPI for loopback audio.

---

## 2. Requirements `[REQ-PLAT-WIN-001]`
1. **WGC Backend (`WgcCaptureBackend`)**:
   - Utilize WinRT `Windows.Graphics.Capture.GraphicsCaptureItem` and `Direct3D11CaptureFramePool`.
   - Support capture cursor visibility toggle and window border toggle (Windows 11+).
2. **DXGI Backend (`DxgiDesktopDuplicationBackend`)**:
   - Utilize `IDXGIOutputDuplication::AcquireNextFrame` on DXGI 1.2+ adapter outputs.
   - Copy hardware surfaces to CPU memory or directly share D3D11 texture pointers with NVENC/Media Foundation.
3. **WASAPI Audio Loopback (`WasapiAudioBackend`)**:
   - Capture default multimedia render endpoint in loopback mode (`AUDCLNT_STREAMFLAGS_LOOPBACK`).
   - Simultaneously capture microphone endpoint and synchronize timestamps.
4. **Hardware Encoder (`MediaFoundationEncoder`)**:
   - Utilize Microsoft Media Foundation Sink Writer with hardware MFTs (NVENC, Intel QuickSync, AMD AMF).
5. **RAII Teardown**:
   - Implement `Drop` on frame pools, capture sessions, D3D11 devices, and WASAPI client handles.

---

## 3. Rust Contracts `[CONTRACT-PLAT-WIN-001]`
```rust
#[cfg(target_os = "windows")]
pub struct WgcCaptureBackend {
    // Private COM / WinRT handles wrapped in RAII
}

#[cfg(target_os = "windows")]
pub struct DxgiDesktopDuplicationBackend {
    // Private D3D11 / DXGI handles wrapped in RAII
}

#[cfg(target_os = "windows")]
pub struct WasapiAudioBackend {
    // Private WASAPI audio clients wrapped in RAII
}
```

---

## 4. Non-Functional Requirements `[NFR-PLAT-WIN-001]`
1. DXGI capture must maintain 60–120 FPS with frame acquisition time < 2ms.
2. GPU resources must release within 100ms when recording stops or is dropped.
3. Every `unsafe` block interacting with COM/Win32 must have a `// SAFETY:` invariant explanation.

---

## 5. Verification Criteria `[TEST-PLAT-WIN-001]`
1. Monitor enumeration test validating retrieval of primary and secondary displays.
2. RAII drop test ensuring no leaked COM references upon dropping capture backend.
