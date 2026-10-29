# SPECIFICATION: High-Definition Animated GIF Exporter
Document ID: `[SPEC-FEAT-GIF-001]`
Status: APPROVED / ACTIVE
Version: 0.1.0
Crate Scope: `rec-core`, `rec-engine`, `rec-app`

---

## 1. Purpose and Scope
This specification defines the architecture, domain models, quantization/LZW algorithms, and UI integration for exporting recorded video clips (`.avi`) or frame sequences into high-definition animated `.gif` files (`5–15` seconds), ideal for GitHub Issues, Pull Requests, Slack, Discord, and documentation workflows.

---

## 2. Functional Requirements (`[REQ-GIF-001]`)

- **`[REQ-GIF-001.1]` Pure Safe Rust GIF89a Encoder**:
  - Implement a 100% memory-safe (`#![forbid(unsafe_code)]`) GIF89a encoder in `rec-core::gif_exporter` producing standards-compliant `.gif` files with `GIF89a` header, Logical Screen Descriptor, per-frame 256-color Adaptive Local Color Tables, `NETSCAPE2.0` Application Extension for infinite looping, Graphic Control Extensions for frame timing, and LZW-compressed image data sub-blocks.
- **`[REQ-GIF-001.2]` Adaptive Median-Cut Palette Quantization, Clamped Floyd-Steinberg Dithering & Bicubic Resampling**:
  - Compute a per-frame 256-color adaptive palette via 3D histogram (`32×32×32`) Median-Cut with exact 8-bit RGB centroid averaging, accelerated by a 3D nearest-neighbor lookup table (`32,768` entries) and subtle clamped Floyd-Steinberg error diffusion.
  - Perform resolution scaling using `image::imageops::FilterType::CatmullRom` bicubic resampling to preserve crisp typography and UI edges, defaulting to `GifScale::Scale100` (full native resolution).
- **`[REQ-GIF-001.3]` Configurable Scale, Frame Rate & Duration Window**:
  - Support export resolution scaling via `GifScale` (`Scale50` = 50%, `Scale75` = 75%, `Scale100` = 100%), target frame rate (`10`, `15`, or `30` FPS), and maximum clip duration (`5` to `15` seconds, default `10` seconds) in `AppConfig`.
- **`[REQ-GIF-001.4]` Direct AVI-to-GIF Conversion (`export_avi_to_gif`)**:
  - Extract `00dc` Motion-JPEG video chunks directly from recorded `.avi` containers, apply frame rate decimation to match the target GIF FPS, resize using fast box/bilinear filtering, and encode to `.gif` on disk.
- **`[REQ-GIF-001.5]` One-Click UI Export & Bilingual i18n**:
  - Provide a **"🎞 GIF"** export button in the Recent Recording card on the main Dashboard and GIF export preferences in the Settings -> Output & Format panel, with 100% English/Spanish parity in `rec_core::i18n`.

---

## 3. Architectural Contracts (`[CONTRACT-GIF-001]`)

```rust
/// Resolution scaling preset for animated GIF export (`[REQ-GIF-001.3]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GifScale {
    /// 50% width and height (compact file size for GitHub/Slack).
    Scale50,
    /// 75% width and height (balanced HD clarity and size).
    #[default]
    Scale75,
    /// 100% original resolution.
    Scale100,
}

/// Configuration parameters for animated GIF export (`[CONTRACT-GIF-001]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GifExportOptions {
    /// Target playback frame rate in frames per second (10, 15, or 30).
    pub fps: u32,
    /// Resolution scaling factor.
    pub scale: GifScale,
    /// Maximum clip duration in seconds (5..=15).
    pub max_duration_secs: u32,
    /// Whether the animation loops infinitely (`NETSCAPE2.0` extension).
    pub loop_infinite: bool,
}
```

---

## 4. Non-Functional Requirements (`[NFR-GIF-001]`)

- **`[NFR-GIF-001.1]` Zero Unsafe Code**: `rec-core::gif_exporter` must compile under `#![forbid(unsafe_code)]` with zero `unwrap()` or `expect()` in production paths.
- **`[NFR-GIF-001.2]` Bounded Memory Usage**: AVI-to-GIF conversion must stream frame-by-frame without loading all uncompressed frames into RAM simultaneously.

---

## 5. Verification Matrix (`[TEST-GIF-001]`)

| Test ID | Test Function | Verification Target |
| :--- | :--- | :--- |
| `[TEST-GIF-001.1]` | `test_gif_encoder_valid_header_and_trailer` | Verifies `GIF89a` header, `NETSCAPE2.0` loop block, LZW sub-blocks, and `0x3B` trailer on synthetic RGBA frames. |
| `[TEST-GIF-001.2]` | `test_export_avi_to_gif_end_to_end` | Records a multi-frame `.avi` file via `AviWriter` and converts it to `.gif` via `export_avi_to_gif`, verifying non-empty valid GIF89a output and scaled dimensions. |
| `[TEST-GIF-001.3]` | `test_gif_scale_dimensions` | Verifies `GifScale::Scale50`, `Scale75`, and `Scale100` dimension calculations and clamping. |

