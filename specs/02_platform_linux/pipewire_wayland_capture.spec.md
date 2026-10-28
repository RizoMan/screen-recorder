# SPECIFICATION: LINUX WAYLAND PIPEWIRE & X11 CAPTURE
Spec ID: `[SPEC-PLAT-LIN-001]`
Status: APPROVED
Domain: Linux Platform Implementation

---

## 1. Overview
Defines screen capture and audio recording mechanisms for Linux using modern Wayland via PipeWire & `xdg-desktop-portal` (Screencast portal), with X11 fallback (XSHM / XDamage) and VA-API hardware encoding.

---

## 2. Requirements `[REQ-PLAT-LIN-001]`
1. **Wayland Portal & PipeWire**:
   - Request screencast session via DBus `org.freedesktop.portal.ScreenCast`.
   - Negotiate DMA-BUF or shared memory stream over PipeWire `pw_stream`.
2. **X11 Fallback**:
   - Fall back to XSHM (MIT-SHM) / XDamage on X11 environments when PipeWire portal is unavailable.
3. **Audio Capture**:
   - PipeWire audio node tap for desktop audio monitor + ALSA/Pulse microphone capture.
4. **Hardware Acceleration**:
   - VA-API (libva) hardware encoding for H.264/HEVC.
5. **RAII Teardown**:
   - Automatically close DBus portal session and destroy PipeWire stream on `Drop`.

---

## 3. Rust Contracts `[CONTRACT-PLAT-LIN-001]`
```rust
pub struct PipeWireCaptureBackend {
    // Pipewire loop and stream handles
}

pub struct X11CaptureBackend {
    // XCB / X11 connection and shared memory handles
}
```

---

## 4. Non-Functional Requirements `[NFR-PLAT-LIN-001]`
1. DMA-BUF zero-copy delivery when GPU acceleration is active.
2. Graceful fallback from Wayland portal to X11 when running under X11 desktop sessions.

---

## 5. Verification Criteria `[TEST-PLAT-LIN-001]`
1. Compilation check under Linux toolchain and stub fallback validation on non-Linux platforms.
2. PipeWire stream state transition tests.
