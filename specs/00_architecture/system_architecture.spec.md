# SPECIFICATION: SYSTEM ARCHITECTURE, TOPOLOGY & PIPELINE
Spec ID: `[SPEC-ARCH-001]`
Status: APPROVED
Domain: Core Architecture & Workspace Topology

---

## 1. Overview
This specification defines the multi-crate workspace topology, data flow pipelines, thread model, and Dependency Inversion Principle (DIP) applied to the Screen Recorder system.

---

## 2. Workspace Crates & DIP Topology `[REQ-ARCH-001]`
The workspace is segregated into 6 decoupled crates:
1. `rec-core`: Pure domain models, audio mixing, zero-copy ring buffers, persistent sanitized configuration, bilingual i18n. No kernel or OS dependencies. `#![forbid(unsafe_code)]`.
2. `rec-platform-windows`: WinRT WGC, DXGI Desktop Duplication, WASAPI audio loopback, Media Foundation hardware encoder.
3. `rec-platform-linux`: Wayland PipeWire portal screencast, X11 fallback (XSHM/XDamage), PipeWire/PulseAudio loopback.
4. `rec-platform-macos`: ScreenCaptureKit capture with window exclusion, VideoToolbox encoder, CoreAudio loopback.
5. `rec-engine`: Pipeline orchestrator, DIP traits (`CaptureBackend`, `AudioBackend`, `EncoderBackend`, `MuxerBackend`), multi-threaded queue orchestration, Instant Replay buffer. `#![forbid(unsafe_code)]`.
6. `rec-app`: GUI (eframe/egui) with dark mode, interactive area selector, transparent HUD, system tray, and CLI interface.

```
[rec-app] ──(Injects concrete backend)──> [rec-engine]
                                                 │
                   ┌─────────────────────────────┼─────────────────────────────┐
                   │ depends on traits           │ depends on traits           │ depends on models
                   ▼                             ▼                             ▼
       [rec-platform-windows]          [rec-platform-linux]               [rec-core]
       [rec-platform-macos  ]
```

---

## 3. Multi-Threaded Streaming Pipeline `[REQ-ARCH-002]`
To maintain 60–120 FPS capture without frame tearing, memory leaks, or thread contention:
1. **Capture Thread (OS-specific)**: Receives native frames from GPU/OS, packages them into `VideoFrame` with nanosecond PTS timestamps, and pushes them into an SPSC lock-free ring buffer.
2. **Audio Capture Thread**: Collects PCM audio frames (WASAPI/PipeWire/CoreAudio), resamples to 48kHz stereo, mixes system audio and mic, and pushes into an audio queue.
3. **Encoding Thread**: Reads from video and audio ring buffers, invokes `EncoderBackend` (hardware NVENC/QSV/AMF/VT or software fallback), producing compressed packets (H.264/HEVC/AAC).
4. **Muxer Thread**: Multiplexes compressed packets into the destination container (MP4/MKV) and writes to disk.
5. **Instant Replay Buffer (Optional branch)**: Simultaneously retains the last N seconds of encoded packets or raw frames in an in-memory circular buffer.

---

## 4. Rust Contracts `[CONTRACT-ARCH-001]`
```rust
pub enum CaptureTarget {
    FullScreen { monitor_index: usize },
    Window { window_id: u64 },
    Area { x: u32, y: u32, width: u32, height: u32 },
}

pub trait CaptureBackend: Send + Sync {
    fn start_capture(
        &mut self,
        target: CaptureTarget,
        frame_tx: crossbeam_channel::Sender<rec_core::frame::VideoFrame>,
    ) -> Result<(), rec_engine::errors::CaptureError>;

    fn stop_capture(&mut self) -> Result<(), rec_engine::errors::CaptureError>;
}
```

---

## 5. Non-Functional Requirements `[NFR-ARCH-001]`
1. Frame delivery latency must be under 16ms (for 60 FPS) and under 8.33ms (for 120 FPS).
2. Pure crates (`rec-core`, `rec-engine`) must have `#![forbid(unsafe_code)]`.
3. CPU overhead during 1080p 60fps recording on hardware encoder must not exceed 5% total CPU utilization.
4. Total memory usage during continuous recording must remain constant (no memory leaks).

---

## 6. Verification Criteria `[TEST-ARCH-001]`
1. Unit tests verifying crate dependency graph contains no cyclic dependencies.
2. Pipeline tests verifying frame transmission across threads without dropped frames under standard loads.
