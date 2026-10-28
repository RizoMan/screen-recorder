# SPECIFICATION: RECORDER GUI, AREA SELECTOR & FLOATING HUD
Spec ID: `[SPEC-GUI-001]`
Status: APPROVED
Domain: User Interface & Presentation

---

## 1. Overview
Defines the modern, dark-mode native user interface built on `eframe`/`egui`. Includes a dashboard, an interactive translucent area selection tool, a non-intrusive floating recording HUD with an audio VU meter, system tray integration, and an interactive CLI mode.

---

## 2. Requirements `[REQ-GUI-001]`
1. **Modern Dark Theme UI**:
   - Sleek dark palette with accent colors, crisp vector typography, and micro-animations.
   - Zero burned strings: all labels and tooltips rendered via `rec_core::i18n`.
2. **Interactive Area Selector**:
   - Transparent fullscreen overlay enabling rectangular drag selection.
   - Live dimension tags (width x height) and pixel grid snapping.
3. **Floating Recording HUD**:
   - Ultra-compact floating pill widget displayed while recording.
   - Displays: recording duration timer (HH:MM:SS), blinking red recording indicator, live dual-channel audio VU meter, Pause/Resume button, Stop button, and Instant Replay trigger button.
   - Always-on-top, draggable, with window exclusion so it is not captured in the recording.
4. **System Tray**:
   - Persistent tray icon with quick actions (Start, Stop, Pause, Instant Replay, Exit).
5. **Interactive CLI Mode**:
   - Command-line arguments: `--record`, `--monitor <INDEX>`, `--area <X,Y,W,H>`, `--output <PATH>`, `--duration <SECS>`.

---

## 3. Rust Contracts `[CONTRACT-GUI-001]`
```rust
pub struct RecorderGuiApp {
    // UI state, engine controller channel, i18n language
}

pub struct FloatingHudState {
    pub is_recording: bool,
    pub is_paused: bool,
    pub elapsed_seconds: u64,
    pub audio_peak_l: f32,
    pub audio_peak_r: f32,
}
```

---

## 4. Non-Functional Requirements `[NFR-GUI-001]`
1. HUD frame rendering overhead must not exceed 2% CPU.
2. Responsiveness: clicks on Pause/Stop must dispatch to the recording pipeline within < 10ms.

---

## 5. Verification Criteria `[TEST-GUI-001]`
1. UI unit tests verifying HUD time formatting helper functions.
2. CLI argument parsing tests for `--record` and area arguments.
