# SPECIFICATION: MACOS SCREENCAPTUREKIT & VIDEOTOOLBOX
Spec ID: `[SPEC-PLAT-MAC-001]`
Status: APPROVED
Domain: macOS Platform Implementation

---

## 1. Overview
Defines screen capture and audio recording mechanisms for macOS (macOS 12.3+) utilizing ScreenCaptureKit (SCKit) for hardware-accelerated desktop/window capture with exclusion lists, VideoToolbox for hardware H.264/HEVC encoding, and CoreAudio for system audio + mic capture.

---

## 2. Requirements `[REQ-PLAT-MAC-001]`
1. **ScreenCaptureKit Capture**:
   - Query shareable content (`SCShareableContent`).
   - Create stream (`SCStream`) with configuration specifying FPS, pixel format (BGRA/420v), and window exclusion lists (to prevent the recording HUD or app from capturing itself).
2. **Audio Capture**:
   - Capture system audio via ScreenCaptureKit audio stream or CoreAudio HAL loopback tap.
   - Microphone capture via AVCaptureDevice / CoreAudio.
3. **Hardware Encoding**:
   - VideoToolbox (`VTCompressionSession`) for hardware H.264 / HEVC encoding.
4. **RAII Teardown**:
   - Stop and release `SCStream` and compression session automatically on `Drop`.

---

## 3. Rust Contracts `[CONTRACT-PLAT-MAC-001]`
```rust
pub struct ScreenCaptureKitBackend {
    // SCStream and delegation state
}

pub struct CoreAudioBackend {
    // AudioUnit / CoreAudio stream state
}
```

---

## 4. Non-Functional Requirements `[NFR-PLAT-MAC-001]`
1. Metal / IOSurface zero-copy pipeline into VideoToolbox.
2. Latency < 16ms for 60 FPS recording.

---

## 5. Verification Criteria `[TEST-PLAT-MAC-001]`
1. Compilation check and stub validation on non-macOS platforms.
2. Permission check validation for screen recording entitlements (`CGPreflightScreenCaptureAccess`).
