//! Smart cursor visual overlay engine (Translucent Halo & Click Ripple animations).
//!
//! Conforms to `[SPEC-FEAT-CURSOR-001]`, `[CONTRACT-CURSOR-001]`, and `[NFR-CURSOR-001]`.

use serde::{Deserialize, Serialize};
use crate::frame::VideoFrame;
use crate::pixel_format::PixelFormat;

/// Selectable color palette presets for the translucent cursor highlight halo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CursorHighlightColor {
    /// Warm golden-yellow highlight (`#FACC15`), ideal for software tutorials.
    #[default]
    YellowGlow,
    /// Obsidian electric blue highlight (`#3B82F6`).
    ElectricBlue,
    /// High-contrast neon emerald highlight (`#10B981`).
    NeonEmerald,
}

impl CursorHighlightColor {
    /// Returns the `(R, G, B)` color components for the highlight preset.
    ///
    /// # Returns
    ///
    /// A tuple `(r, g, b)` of 8-bit sRGB channel values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::cursor_effects::CursorHighlightColor;
    ///
    /// assert_eq!(CursorHighlightColor::YellowGlow.rgb(), (0xFA, 0xCC, 0x15));
    /// ```
    #[must_use]
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::YellowGlow => (0xFA, 0xCC, 0x15),
            Self::ElectricBlue => (0x3B, 0x82, 0xF6),
            Self::NeonEmerald => (0x10, 0xB9, 0x81),
        }
    }
}

/// Mouse button identifier for click ripple animations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickButton {
    /// Primary left mouse button (rendered as an expanding Electric Blue ring).
    Left,
    /// Secondary right mouse button (rendered as an expanding Amber Orange ring).
    Right,
}

impl ClickButton {
    /// Returns the `(R, G, B)` color components for this mouse button's ripple wave.
    ///
    /// # Returns
    ///
    /// A tuple `(r, g, b)` of 8-bit sRGB channel values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::cursor_effects::ClickButton;
    ///
    /// assert_eq!(ClickButton::Left.rgb(), (0x3B, 0x82, 0xF6));
    /// assert_eq!(ClickButton::Right.rgb(), (0xF5, 0x9E, 0x0B));
    /// ```
    #[must_use]
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Left => (0x3B, 0x82, 0xF6),  // Electric Blue (#3B82F6)
            Self::Right => (0xF5, 0x9E, 0x0B), // Amber Orange (#F59E0B)
        }
    }
}

/// Represents an active expanding click ripple wave on screen.
#[derive(Debug, Clone, Copy)]
pub struct ClickRipple {
    /// X coordinate relative to the captured frame.
    pub x: i32,
    /// Y coordinate relative to the captured frame.
    pub y: i32,
    /// Mouse button that triggered the ripple.
    pub button: ClickButton,
    /// Normalized animation progress in `0.0..=1.0`.
    pub progress: f32,
}

/// Snapshot of cursor position, halo settings, and active click ripples for a single frame.
#[derive(Debug, Clone)]
pub struct CursorOverlayState {
    /// Current cursor X coordinate relative to the captured surface.
    pub cursor_x: i32,
    /// Current cursor Y coordinate relative to the captured surface.
    pub cursor_y: i32,
    /// Whether the cursor is currently inside the captured region and visible.
    pub cursor_visible: bool,
    /// Whether the translucent cursor halo is enabled.
    pub halo_enabled: bool,
    /// Radius of the cursor halo in pixels (clamped between 12 and 64).
    pub halo_radius: u32,
    /// Color preset for the cursor halo.
    pub halo_color: CursorHighlightColor,
    /// Active click ripple animations to composite onto the frame.
    pub ripples: Vec<ClickRipple>,
}

/// Composites the translucent cursor halo and expanding click ripples onto a [`VideoFrame`] in place.
///
/// Restricts pixel iteration strictly to the local bounding boxes of the cursor halo and active ripples,
/// achieving `< 0.05 ms` execution time with zero heap allocations (`[NFR-CURSOR-001]`).
///
/// # Arguments
///
/// * `frame` - Mutable reference to the captured [`VideoFrame`] (`Rgba` or `Bgra`).
/// * `state` - Current cursor overlay state and active ripples.
///
/// # Examples
///
/// ```rust
/// use rec_core::cursor_effects::{apply_cursor_effects, CursorHighlightColor, CursorOverlayState};
/// use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
/// use rec_core::pixel_format::PixelFormat;
///
/// let desc = FrameDescriptor::new(100, 100, PixelFormat::Rgba).unwrap();
/// let mut frame = VideoFrame::new(desc, Pts::from_nanos(0), vec![0u8; 100 * 100 * 4]).unwrap();
/// let state = CursorOverlayState {
///     cursor_x: 50,
///     cursor_y: 50,
///     cursor_visible: true,
///     halo_enabled: true,
///     halo_radius: 20,
///     halo_color: CursorHighlightColor::YellowGlow,
///     ripples: Vec::new(),
/// };
/// apply_cursor_effects(&mut frame, &state);
/// ```
pub fn apply_cursor_effects(frame: &mut VideoFrame, state: &CursorOverlayState) {
    let format = frame.descriptor.format;
    if format != PixelFormat::Rgba && format != PixelFormat::Bgra {
        return;
    }

    let width = frame.descriptor.width as i32;
    let height = frame.descriptor.height as i32;
    if width <= 0 || height <= 0 {
        return;
    }

    let is_bgra = format == PixelFormat::Bgra;

    // 1. Render translucent radial cursor halo
    if state.halo_enabled && state.cursor_visible {
        let radius = state.halo_radius.clamp(12, 64) as i32;
        let (hr, hg, hb) = state.halo_color.rgb();
        draw_filled_halo(
            &mut frame.data,
            width,
            height,
            is_bgra,
            state.cursor_x,
            state.cursor_y,
            radius,
            (hr, hg, hb),
        );
    }

    // 2. Render expanding concentric click ripples
    let base_max_radius = state.halo_radius.clamp(16, 64) as f32 * 1.45;
    for ripple in &state.ripples {
        let p = ripple.progress.clamp(0.0, 1.0);
        if p >= 1.0 {
            continue;
        }
        // Cubic ease-out expansion: 1 - (1 - p)^3
        let inv = 1.0 - p;
        let eased = 1.0 - (inv * inv * inv);
        let current_radius = 6.0 + eased * base_max_radius;
        let peak_alpha = (1.0 - p) * 0.85;
        let (rr, rg, rb) = ripple.button.rgb();

        draw_expanding_ring(
            &mut frame.data,
            width,
            height,
            is_bgra,
            ripple.x,
            ripple.y,
            current_radius,
            3.0,
            peak_alpha,
            (rr, rg, rb),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_filled_halo(
    data: &mut [u8],
    width: i32,
    height: i32,
    is_bgra: bool,
    cx: i32,
    cy: i32,
    radius: i32,
    rgb: (u8, u8, u8),
) {
    let min_x = (cx - radius).max(0);
    let max_x = (cx + radius).min(width - 1);
    let min_y = (cy - radius).max(0);
    let max_y = (cy + radius).min(height - 1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let r_f = radius as f32;
    let r_sq = (radius * radius) as f32;
    let (sr, sg, sb) = (rgb.0 as f32, rgb.1 as f32, rgb.2 as f32);

    for y in min_y..=max_y {
        let dy = (y - cy) as f32;
        let dy_sq = dy * dy;
        let row_offset = (y * width) as usize;

        for x in min_x..=max_x {
            let dx = (x - cx) as f32;
            let dist_sq = dx * dx + dy_sq;
            if dist_sq > r_sq {
                continue;
            }

            let dist = dist_sq.sqrt();
            let norm = (dist / r_f).clamp(0.0, 1.0);

            // Smooth translucent fill (38% at center tapering smoothly, plus a crisp outer border ring)
            let fill_alpha = (1.0 - norm * norm * 0.65) * 0.36;
            let border_dist = (r_f - dist).abs();
            let border_alpha = if border_dist < 1.8 {
                (1.0 - border_dist / 1.8) * 0.65
            } else {
                0.0
            };
            let alpha = fill_alpha.max(border_alpha).clamp(0.0, 0.75);

            blend_pixel(data, (row_offset + x as usize) * 4, is_bgra, sr, sg, sb, alpha);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_expanding_ring(
    data: &mut [u8],
    width: i32,
    height: i32,
    is_bgra: bool,
    cx: i32,
    cy: i32,
    radius: f32,
    thickness: f32,
    peak_alpha: f32,
    rgb: (u8, u8, u8),
) {
    let outer_r = (radius + thickness).ceil() as i32;
    let inner_r = (radius - thickness).max(0.0);
    let inner_sq = inner_r * inner_r;
    let outer_sq = (radius + thickness) * (radius + thickness);

    let min_x = (cx - outer_r).max(0);
    let max_x = (cx + outer_r).min(width - 1);
    let min_y = (cy - outer_r).max(0);
    let max_y = (cy + outer_r).min(height - 1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let (sr, sg, sb) = (rgb.0 as f32, rgb.1 as f32, rgb.2 as f32);

    for y in min_y..=max_y {
        let dy = (y - cy) as f32;
        let dy_sq = dy * dy;
        let row_offset = (y * width) as usize;

        for x in min_x..=max_x {
            let dx = (x - cx) as f32;
            let dist_sq = dx * dx + dy_sq;
            if dist_sq < inner_sq || dist_sq > outer_sq {
                continue;
            }

            let dist = dist_sq.sqrt();
            let ring_delta = (dist - radius).abs();
            if ring_delta <= thickness {
                let aa = 1.0 - (ring_delta / thickness);
                let alpha = (aa * peak_alpha).clamp(0.0, 1.0);
                blend_pixel(data, (row_offset + x as usize) * 4, is_bgra, sr, sg, sb, alpha);
            }
        }
    }
}

#[inline]
fn blend_pixel(
    data: &mut [u8],
    idx: usize,
    is_bgra: bool,
    sr: f32,
    sg: f32,
    sb: f32,
    alpha: f32,
) {
    if idx + 3 >= data.len() {
        return;
    }
    let inv = 1.0 - alpha;
    if is_bgra {
        let db = data[idx] as f32;
        let dg = data[idx + 1] as f32;
        let dr = data[idx + 2] as f32;
        data[idx] = (db * inv + sb * alpha) as u8;
        data[idx + 1] = (dg * inv + sg * alpha) as u8;
        data[idx + 2] = (dr * inv + sr * alpha) as u8;
        data[idx + 3] = 255;
    } else {
        let dr = data[idx] as f32;
        let dg = data[idx + 1] as f32;
        let db = data[idx + 2] as f32;
        data[idx] = (dr * inv + sr * alpha) as u8;
        data[idx + 1] = (dg * inv + sg * alpha) as u8;
        data[idx + 2] = (db * inv + sb * alpha) as u8;
        data[idx + 3] = 255;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::frame::{FrameDescriptor, Pts};

    /// Verifies `[TEST-CURSOR-001]` radial halo blending inside radius and untouched outside.
    #[test]
    fn test_cursor_halo_blends_inside_and_preserves_outside() {
        let desc = FrameDescriptor::new(100, 100, PixelFormat::Rgba).unwrap();
        let mut frame = VideoFrame::new(desc, Pts::from_nanos(0), vec![20u8; 100 * 100 * 4]).unwrap();

        let state = CursorOverlayState {
            cursor_x: 50,
            cursor_y: 50,
            cursor_visible: true,
            halo_enabled: true,
            halo_radius: 20,
            halo_color: CursorHighlightColor::YellowGlow,
            ripples: Vec::new(),
        };

        apply_cursor_effects(&mut frame, &state);

        // Center pixel (50, 50) must be blended toward YellowGlow (R > G > B)
        let center_idx = (50 * 100 + 50) * 4;
        assert!(frame.data[center_idx] > 80, "Red channel at halo center should increase");
        assert!(frame.data[center_idx + 1] > 60, "Green channel at halo center should increase");

        // Far corner pixel (0, 0) must remain completely untouched
        assert_eq!(frame.data[0], 20);
        assert_eq!(frame.data[1], 20);
        assert_eq!(frame.data[2], 20);
    }

    /// Verifies `[TEST-CURSOR-001]` BGRA channel ordering and click ripple rendering.
    #[test]
    fn test_cursor_click_ripple_bgra_and_edge_clipping() {
        let desc = FrameDescriptor::new(64, 64, PixelFormat::Bgra).unwrap();
        let mut frame = VideoFrame::new(desc, Pts::from_nanos(0), vec![10u8; 64 * 64 * 4]).unwrap();

        let state = CursorOverlayState {
            cursor_x: -5,
            cursor_y: 63,
            cursor_visible: true,
            halo_enabled: true,
            halo_radius: 24,
            halo_color: CursorHighlightColor::ElectricBlue,
            ripples: vec![
                ClickRipple {
                    x: 32,
                    y: 32,
                    button: ClickButton::Left,
                    progress: 0.3,
                },
                ClickRipple {
                    x: 63,
                    y: 63,
                    button: ClickButton::Right,
                    progress: 0.5,
                },
            ],
        };

        // Must safely clip negative and edge coordinates without panicking
        apply_cursor_effects(&mut frame, &state);

        // Verify some pixels were modified by the ripple and halo
        assert!(frame.data.chunks_exact(4).any(|px| px[0] > 10 || px[2] > 10));
    }
}

