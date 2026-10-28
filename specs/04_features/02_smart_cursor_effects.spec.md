# SPECIFICATION: SMART CURSOR EFFECTS (HALO & CLICK RIPPLES)
Spec ID: `[SPEC-FEAT-CURSOR-001]`
Status: APPROVED
Domain: Visual Capture Enhancements & Creator UX

---

## 1. Overview
Defines a real-time, zero-allocation visual overlay engine that highlights the mouse cursor with a customizable translucent halo and renders expanding concentric ripple animations on left and right mouse clicks directly onto captured video frames.

---

## 2. Requirements `[REQ-CURSOR-001]`
1. **Translucent Cursor Halo (`[REQ-CURSOR-001.1]`)**:
   - Render a smooth radial alpha-blended highlight circle centered at the cursor coordinates `(x, y)` relative to the active capture surface.
   - Support selectable color presets:
     * `YellowGlow` (`#FACC15` — Classic tutorial highlight).
     * `ElectricBlue` (`#3B82F6` — Obsidian primary accent).
     * `NeonEmerald` (`#10B981` — High-contrast emerald).
   - Configurable halo radius between `12` and `64` pixels (default `28` px).
2. **Click Ripple Animations (`[REQ-CURSOR-001.2]`)**:
   - Detect mouse button press transitions (`Left` and `Right` clicks).
   - Spawn an expanding circular ring animation lasting `350 ms` (`progress` from `0.0` to `1.0`):
     * **Left Click (`ClickButton::Left`)**: Expanding Electric Blue (`#3B82F6`) ring with smooth alpha fade-out.
     * **Right Click (`ClickButton::Right`)**: Expanding Amber Orange (`#F59E0B`) ring with smooth alpha fade-out.
3. **Live Configuration & Bounding-Box Clipping (`[REQ-CURSOR-001.3]`)**:
   - Toggling halo or click ripples in the GUI updates the active recording session in real time.
   - Rendering must clip safely against frame boundaries (`0..width`, `0..height`) when the cursor is near screen edges or outside the captured region.

---

## 3. Rust Contracts `[CONTRACT-CURSOR-001]`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum CursorHighlightColor {
    #[default]
    YellowGlow,
    ElectricBlue,
    NeonEmerald,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickButton {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub struct ClickRipple {
    pub x: i32,
    pub y: i32,
    pub button: ClickButton,
    pub progress: f32,
}

#[derive(Debug, Clone)]
pub struct CursorOverlayState {
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub cursor_visible: bool,
    pub halo_enabled: bool,
    pub halo_radius: u32,
    pub halo_color: CursorHighlightColor,
    pub ripples: Vec<ClickRipple>,
}

pub fn apply_cursor_effects(frame: &mut VideoFrame, state: &CursorOverlayState);
```

---

## 4. Non-Functional Requirements `[NFR-CURSOR-001]`
1. **Sub-0.05ms CPU Budget**: Alpha blending is restricted strictly to the bounding box `[x - R .. x + R] × [y - R .. y + R]`, never scanning the full frame.
2. **Zero Heap Allocations on Hot Path**: `apply_cursor_effects` modifies `VideoFrame::data` in place (`Rgba` and `Bgra` pixel formats) with `#![forbid(unsafe_code)]` in `rec-core`.

---

## 5. Verification Criteria `[TEST-CURSOR-001]`
1. Unit test verifying radial alpha blending modifies pixels inside the halo radius and leaves pixels outside untouched.
2. Unit test verifying both `PixelFormat::Rgba` and `PixelFormat::Bgra` channel ordering for `YellowGlow`, `ElectricBlue`, and `NeonEmerald`.
3. Edge-clipping test verifying safe execution when cursor coordinates are negative or at the extreme bottom-right border of a frame without panicking.

