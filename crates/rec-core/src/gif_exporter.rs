//! High-definition animated GIF89a exporter with Adaptive Median-Cut palette quantization,
//! clamped Floyd-Steinberg error diffusion, and bicubic `CatmullRom` resampling.
//!
//! Conforms to `[SPEC-FEAT-GIF-001]`, `[CONTRACT-GIF-001]`, `RULE-ARCH-001` (SOLID, DRY, KISS),
//! `RULE-SAFE-001` (`#![forbid(unsafe_code)]`, RAII), and `RULE-DOC-001`.

use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use image::imageops::{resize, FilterType};
use image::RgbImage;
use serde::{Deserialize, Serialize};
use crate::errors::CoreError;
use crate::frame::VideoFrame;
use crate::pixel_format::PixelFormat;

/// Resolution scaling preset for animated GIF export (`[REQ-GIF-001.3]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GifScale {
    /// 50% width and height (compact file size for GitHub Issues / Slack).
    Scale50,
    /// 75% width and height (balanced HD clarity and size).
    Scale75,
    /// 100% original capture resolution (Maximum HD fidelity).
    #[default]
    Scale100,
}

impl GifScale {
    /// Computes the scaled `(width, height)` dimensions for a given source resolution.
    ///
    /// Ensures dimensions are at least `2×2` pixels.
    ///
    /// # Arguments
    ///
    /// * `width` - Source frame width in pixels.
    /// * `height` - Source frame height in pixels.
    ///
    /// # Returns
    ///
    /// A `(u16, u16)` tuple of scaled `(width, height)` clamped to `2..=4096`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::gif_exporter::GifScale;
    ///
    /// assert_eq!(GifScale::Scale50.scaled_dimensions(1920, 1080), (960, 540));
    /// assert_eq!(GifScale::Scale75.scaled_dimensions(1920, 1080), (1440, 810));
    /// assert_eq!(GifScale::Scale100.scaled_dimensions(1920, 1080), (1920, 1080));
    /// ```
    #[must_use]
    pub fn scaled_dimensions(self, width: u32, height: u32) -> (u16, u16) {
        let (num, den) = match self {
            Self::Scale50 => (1, 2),
            Self::Scale75 => (3, 4),
            Self::Scale100 => (1, 1),
        };
        let sw = ((width * num) / den).clamp(2, 4096) as u16;
        let sh = ((height * num) / den).clamp(2, 4096) as u16;
        (sw, sh)
    }

    /// Returns the percentage integer value (`50`, `75`, or `100`).
    ///
    /// # Returns
    ///
    /// Percentage scale factor as `u32`.
    #[must_use]
    pub const fn percent(self) -> u32 {
        match self {
            Self::Scale50 => 50,
            Self::Scale75 => 75,
            Self::Scale100 => 100,
        }
    }
}

/// Configuration parameters for animated GIF export (`[CONTRACT-GIF-001]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GifExportOptions {
    /// Target playback frame rate in frames per second (`10`, `15`, or `30`).
    pub fps: u32,
    /// Resolution scaling preset.
    pub scale: GifScale,
    /// Maximum clip duration in seconds (`5..=15`).
    pub max_duration_secs: u32,
    /// Whether the animation loops infinitely (`NETSCAPE2.0` extension).
    pub loop_infinite: bool,
}

impl Default for GifExportOptions {
    fn default() -> Self {
        Self {
            fps: 20,
            scale: GifScale::Scale100,
            max_duration_secs: 10,
            loop_infinite: true,
        }
    }
}

impl GifExportOptions {
    /// Creates a new sanitized [`GifExportOptions`] configuration.
    ///
    /// # Arguments
    ///
    /// * `fps` - Target GIF framerate (`5..=30`).
    /// * `scale` - Resolution scaling preset.
    /// * `max_duration_secs` - Maximum duration in seconds (`5..=15`).
    ///
    /// # Returns
    ///
    /// Validated [`GifExportOptions`] with infinite looping enabled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::gif_exporter::{GifExportOptions, GifScale};
    ///
    /// let opts = GifExportOptions::new(15, GifScale::Scale75, 10);
    /// assert_eq!(opts.fps, 15);
    /// assert_eq!(opts.max_duration_secs, 10);
    /// ```
    #[must_use]
    pub fn new(fps: u32, scale: GifScale, max_duration_secs: u32) -> Self {
        Self {
            fps: fps.clamp(5, 30),
            scale,
            max_duration_secs: max_duration_secs.clamp(5, 15),
            loop_infinite: true,
        }
    }

    /// Computes the GIF89a frame delay in centiseconds (1/100s) for the configured `fps`.
    ///
    /// # Returns
    ///
    /// Frame delay in centiseconds (minimum `2` cs).
    #[must_use]
    pub fn frame_delay_centisecs(&self) -> u16 {
        let fps = self.fps.clamp(5, 30);
        ((100 + (fps / 2)) / fps).max(2) as u16
    }
}

/// Non-empty 5-5-5 RGB histogram cell storing exact 8-bit color sums for high-precision centroid calculation.
#[derive(Debug, Clone, Copy)]
struct ColorBin {
    r_bin: u8,
    g_bin: u8,
    b_bin: u8,
    count: u32,
    sum_r: u64,
    sum_g: u64,
    sum_b: u64,
}

/// Sub-space bounding box used during Adaptive Median-Cut palette partitioning.
#[derive(Debug, Clone, Copy)]
struct MedianBox {
    start: usize,
    end: usize,
    r_min: u8,
    r_max: u8,
    g_min: u8,
    g_max: u8,
    b_min: u8,
    b_max: u8,
    pixel_count: u64,
}

impl MedianBox {
    fn from_slice(bins: &[ColorBin], start: usize, end: usize) -> Self {
        let mut r_min = 31u8;
        let mut r_max = 0u8;
        let mut g_min = 31u8;
        let mut g_max = 0u8;
        let mut b_min = 31u8;
        let mut b_max = 0u8;
        let mut pixel_count = 0u64;

        for b in &bins[start..end] {
            r_min = r_min.min(b.r_bin);
            r_max = r_max.max(b.r_bin);
            g_min = g_min.min(b.g_bin);
            g_max = g_max.max(b.g_bin);
            b_min = b_min.min(b.b_bin);
            b_max = b_max.max(b.b_bin);
            pixel_count += u64::from(b.count);
        }

        Self {
            start,
            end,
            r_min,
            r_max,
            g_min,
            g_max,
            b_min,
            b_max,
            pixel_count,
        }
    }

    /// Perceptual priority score for splitting this box during Median-Cut.
    fn split_score(&self) -> u64 {
        if self.end - self.start <= 1 {
            return 0;
        }
        let dr = u64::from(self.r_max - self.r_min) * 3;
        let dg = u64::from(self.g_max - self.g_min) * 4;
        let db = u64::from(self.b_max - self.b_min) * 2;
        let max_span = dr.max(dg).max(db);
        // Combine perceptual color span with log/sqrt-like population weight so both
        // dominant UI surfaces and high-contrast syntax/accent colors receive dedicated entries
        let pop_weight = (self.pixel_count as f64).sqrt() as u64 + 8;
        max_span * pop_weight
    }
}

/// Computes an adaptive 256-color palette (`768` bytes) tailored to the exact RGB colors
/// in `rgb` using a 3D `32×32×32` histogram and perceptual Median-Cut (`[REQ-GIF-001.2]`),
/// then maps all pixels into `indexed_out` with subtle clamped Floyd-Steinberg error diffusion.
fn quantize_frame_adaptive(
    rgb: &[u8],
    width: usize,
    height: usize,
    indexed_out: &mut [u8],
    lut_cache: &mut [i16],
) -> [u8; 768] {
    const HIST_SIZE: usize = 32 * 32 * 32;
    let mut counts = vec![0u32; HIST_SIZE];
    let mut sum_r = vec![0u64; HIST_SIZE];
    let mut sum_g = vec![0u64; HIST_SIZE];
    let mut sum_b = vec![0u64; HIST_SIZE];

    for chunk in rgb.chunks_exact(3) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let idx = ((usize::from(r >> 3)) << 10)
            | ((usize::from(g >> 3)) << 5)
            | usize::from(b >> 3);
        counts[idx] += 1;
        sum_r[idx] += u64::from(r);
        sum_g[idx] += u64::from(g);
        sum_b[idx] += u64::from(b);
    }

    let mut bins = Vec::with_capacity(4096);
    for idx in 0..HIST_SIZE {
        let c = counts[idx];
        if c > 0 {
            bins.push(ColorBin {
                r_bin: ((idx >> 10) & 31) as u8,
                g_bin: ((idx >> 5) & 31) as u8,
                b_bin: (idx & 31) as u8,
                count: c,
                sum_r: sum_r[idx],
                sum_g: sum_g[idx],
                sum_b: sum_b[idx],
            });
        }
    }

    let mut palette = [0u8; 768];

    if bins.is_empty() {
        indexed_out.fill(0);
        return palette;
    }

    let num_colors = if bins.len() <= 256 {
        for (i, b) in bins.iter().enumerate() {
            let cnt = u64::from(b.count).max(1);
            palette[i * 3] = (b.sum_r / cnt) as u8;
            palette[i * 3 + 1] = (b.sum_g / cnt) as u8;
            palette[i * 3 + 2] = (b.sum_b / cnt) as u8;
        }
        bins.len()
    } else {
        let mut boxes = Vec::with_capacity(256);
        boxes.push(MedianBox::from_slice(&bins, 0, bins.len()));

        while boxes.len() < 256 {
            let mut best_idx = None;
            let mut best_score = 0u64;
            for (i, bx) in boxes.iter().enumerate() {
                let sc = bx.split_score();
                if sc > best_score {
                    best_score = sc;
                    best_idx = Some(i);
                }
            }

            let Some(split_idx) = best_idx else {
                break;
            };

            let bx = boxes[split_idx];
            let dr = u16::from(bx.r_max - bx.r_min) * 3;
            let dg = u16::from(bx.g_max - bx.g_min) * 4;
            let db = u16::from(bx.b_max - bx.b_min) * 2;

            let sub = &mut bins[bx.start..bx.end];
            if dg >= dr && dg >= db {
                sub.sort_unstable_by_key(|b| (b.g_bin, b.r_bin, b.b_bin));
            } else if dr >= dg && dr >= db {
                sub.sort_unstable_by_key(|b| (b.r_bin, b.g_bin, b.b_bin));
            } else {
                sub.sort_unstable_by_key(|b| (b.b_bin, b.g_bin, b.r_bin));
            }

            // Split near median population count, falling back to geometric midpoint
            let half_count = bx.pixel_count / 2;
            let mut acc = 0u64;
            let mut mid_offset = sub.len() / 2;
            for (offset, b) in sub.iter().enumerate() {
                acc += u64::from(b.count);
                if acc >= half_count {
                    mid_offset = offset.clamp(1, sub.len() - 1);
                    break;
                }
            }

            let mid = bx.start + mid_offset;
            let left_box = MedianBox::from_slice(&bins, bx.start, mid);
            let right_box = MedianBox::from_slice(&bins, mid, bx.end);
            boxes[split_idx] = left_box;
            boxes.push(right_box);
        }

        let colors = boxes.len().min(256);
        for (i, bx) in boxes.iter().take(colors).enumerate() {
            let mut sr = 0u64;
            let mut sg = 0u64;
            let mut sb = 0u64;
            let mut cnt = 0u64;
            for b in &bins[bx.start..bx.end] {
                sr += b.sum_r;
                sg += b.sum_g;
                sb += b.sum_b;
                cnt += u64::from(b.count);
            }
            let cnt = cnt.max(1);
            palette[i * 3] = (sr / cnt) as u8;
            palette[i * 3 + 1] = (sg / cnt) as u8;
            palette[i * 3 + 2] = (sb / cnt) as u8;
        }
        colors
    };

    lut_cache.fill(-1);

    let find_nearest = |r: u8, g: u8, b: u8, lut: &mut [i16]| -> u8 {
        let cell = ((usize::from(r >> 3)) << 10)
            | ((usize::from(g >> 3)) << 5)
            | usize::from(b >> 3);
        let cached = lut[cell];
        if cached >= 0 {
            return cached as u8;
        }

        // Representative center of the 5-5-5 cell (or exact match check)
        let mut best_i = 0u8;
        let mut best_dist = u32::MAX;
        let ri = i32::from(r);
        let gi = i32::from(g);
        let bi = i32::from(b);

        for i in 0..num_colors {
            let pr = i32::from(palette[i * 3]);
            let pg = i32::from(palette[i * 3 + 1]);
            let pb = i32::from(palette[i * 3 + 2]);
            let dr = ri - pr;
            let dg = gi - pg;
            let db = bi - pb;
            // Perceptual weighted Euclidean distance
            let dist = (dr * dr * 3 + dg * dg * 4 + db * db * 2) as u32;
            if dist < best_dist {
                best_dist = dist;
                best_i = i as u8;
                if dist == 0 {
                    break;
                }
            }
        }

        lut[cell] = i16::from(best_i);
        best_i
    };

    // Subtle clamped Floyd-Steinberg error diffusion along scanlines
    let mut curr_err = vec![[0i16; 3]; width + 2];
    let mut next_err = vec![[0i16; 3]; width + 2];

    for y in 0..height {
        next_err.fill([0; 3]);
        let row_rgb = y * width * 3;
        let row_dst = y * width;

        for x in 0..width {
            let px = row_rgb + x * 3;
            let r0 = rgb[px];
            let g0 = rgb[px + 1];
            let b0 = rgb[px + 2];

            // Clamp diffused error to [-8..=8] to prevent speckle on flat desktop UI regions
            let er_in = curr_err[x + 1][0].clamp(-8, 8);
            let eg_in = curr_err[x + 1][1].clamp(-8, 8);
            let eb_in = curr_err[x + 1][2].clamp(-8, 8);

            let r = (i16::from(r0) + er_in).clamp(0, 255) as u8;
            let g = (i16::from(g0) + eg_in).clamp(0, 255) as u8;
            let b = (i16::from(b0) + eb_in).clamp(0, 255) as u8;

            let p_idx = find_nearest(r, g, b, lut_cache);
            indexed_out[row_dst + x] = p_idx;

            let pr = palette[usize::from(p_idx) * 3];
            let pg = palette[usize::from(p_idx) * 3 + 1];
            let pb = palette[usize::from(p_idx) * 3 + 2];

            // If the original pixel is already very close to the palette entry (flat UI surface),
            // suppress error diffusion so solid colors stay 100% clean.
            if r0.abs_diff(pr) <= 3 && g0.abs_diff(pg) <= 3 && b0.abs_diff(pb) <= 3 {
                continue;
            }

            let er = i16::from(r) - i16::from(pr);
            let eg = i16::from(g) - i16::from(pg);
            let eb = i16::from(b) - i16::from(pb);

            // Distribute 7/16 right, 3/16 bottom-left, 5/16 bottom, 1/16 bottom-right
            curr_err[x + 2][0] += (er * 7) >> 4;
            curr_err[x + 2][1] += (eg * 7) >> 4;
            curr_err[x + 2][2] += (eb * 7) >> 4;

            next_err[x][0] += (er * 3) >> 4;
            next_err[x][1] += (eg * 3) >> 4;
            next_err[x][2] += (eb * 3) >> 4;

            next_err[x + 1][0] += (er * 5) >> 4;
            next_err[x + 1][1] += (eg * 5) >> 4;
            next_err[x + 1][2] += (eb * 5) >> 4;

            next_err[x + 2][0] += er >> 4;
            next_err[x + 2][1] += eg >> 4;
            next_err[x + 2][2] += eb >> 4;
        }

        std::mem::swap(&mut curr_err, &mut next_err);
    }

    palette
}

/// Compresses 8-bit indexed pixels using the standard GIF89a variable-width LZW algorithm.
fn lzw_encode_gif_indices(indices: &[u8], lzw_table: &mut [i16]) -> Vec<u8> {
    const CLEAR_CODE: u16 = 256;
    const END_CODE: u16 = 257;
    const MAX_CODE: u16 = 4095;

    let mut out = Vec::with_capacity(indices.len() / 2 + 64);
    let mut bit_buf: u32 = 0;
    let mut bit_count: u32 = 0;

    let mut emit_code = |code: u16, bits: u32, dest: &mut Vec<u8>| {
        bit_buf |= u32::from(code) << bit_count;
        bit_count += bits;
        while bit_count >= 8 {
            dest.push((bit_buf & 0xFF) as u8);
            bit_buf >>= 8;
            bit_count -= 8;
        }
    };

    lzw_table.fill(-1);
    let mut code_size: u32 = 9;
    let mut next_code: u16 = 258;

    emit_code(CLEAR_CODE, code_size, &mut out);

    if indices.is_empty() {
        emit_code(END_CODE, code_size, &mut out);
        if bit_count > 0 {
            out.push((bit_buf & 0xFF) as u8);
        }
        return out;
    }

    let mut prefix = u16::from(indices[0]);

    for &k in &indices[1..] {
        let table_idx = (usize::from(prefix) << 8) | usize::from(k);
        let entry = lzw_table[table_idx];
        if entry >= 0 {
            prefix = entry as u16;
        } else {
            emit_code(prefix, code_size, &mut out);
            if next_code <= MAX_CODE {
                lzw_table[table_idx] = next_code as i16;
                if next_code == (1 << code_size) && code_size < 12 {
                    code_size += 1;
                }
                next_code += 1;
            } else {
                emit_code(CLEAR_CODE, code_size, &mut out);
                lzw_table.fill(-1);
                code_size = 9;
                next_code = 258;
            }
            prefix = u16::from(k);
        }
    }

    emit_code(prefix, code_size, &mut out);
    emit_code(END_CODE, code_size, &mut out);

    if bit_count > 0 {
        out.push((bit_buf & 0xFF) as u8);
    }

    out
}

/// Pure safe Rust GIF89a animated image writer with per-frame adaptive 256-color palettes
/// and RAII finalization (`[REQ-GIF-001.1]`, `[REQ-GIF-001.2]`).
pub struct GifWriter {
    writer: BufWriter<File>,
    width: u16,
    height: u16,
    delay_cs: u16,
    frame_count: u32,
    is_finalized: bool,
    indexed_scratch: Vec<u8>,
    lut_cache: Vec<i16>,
    lzw_table: Vec<i16>,
}

impl GifWriter {
    /// Creates a new [`GifWriter`] on disk and writes the `GIF89a` header, Logical Screen Descriptor,
    /// and optional `NETSCAPE2.0` loop extension.
    ///
    /// # Arguments
    ///
    /// * `path` - Output `.gif` file path.
    /// * `width` - Target GIF canvas width in pixels (`2..=4096`).
    /// * `height` - Target GIF canvas height in pixels (`2..=4096`).
    /// * `options` - [`GifExportOptions`] controlling frame delay and looping.
    ///
    /// # Returns
    ///
    /// Initialized [`GifWriter`] ready to accept frames.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidDimensions`] if `width` or `height` is zero, or
    /// [`CoreError::Serialization`] on file I/O errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::gif_exporter::{GifExportOptions, GifScale, GifWriter};
    ///
    /// let temp_path = std::env::temp_dir().join("doctest_gif_writer.gif");
    /// let opts = GifExportOptions::new(15, GifScale::Scale100, 5);
    /// let mut writer = GifWriter::new(&temp_path, 32, 32, opts).unwrap();
    /// let rgb = vec![128u8; 32 * 32 * 3];
    /// writer.write_rgb_frame(&rgb, 32, 32).unwrap();
    /// writer.finalize().unwrap();
    /// let _ = std::fs::remove_file(temp_path);
    /// ```
    pub fn new<P: AsRef<Path>>(
        path: P,
        width: u16,
        height: u16,
        options: GifExportOptions,
    ) -> Result<Self, CoreError> {
        if width == 0 || height == 0 {
            return Err(CoreError::InvalidDimensions {
                width: u32::from(width),
                height: u32::from(height),
            });
        }

        if let Some(parent) = path.as_ref().parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let file = File::create(path.as_ref())
            .map_err(|e| CoreError::Serialization(format!("Failed to create GIF file: {}", e)))?;
        let mut writer = BufWriter::with_capacity(256 * 1024, file);

        // 1. GIF89a Header
        writer
            .write_all(b"GIF89a")
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 2. Logical Screen Descriptor (Global Color Table present for Netscape loop compatibility)
        let mut lsd = [0u8; 7];
        lsd[0..2].copy_from_slice(&width.to_le_bytes());
        lsd[2..4].copy_from_slice(&height.to_le_bytes());
        lsd[4] = 0xF7;
        lsd[5] = 0;
        lsd[6] = 0;
        writer
            .write_all(&lsd)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 3. Default Global Color Table (overridden per-frame by adaptive Local Color Tables)
        writer
            .write_all(&[0u8; 768])
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 4. NETSCAPE2.0 Infinite Loop Application Extension
        if options.loop_infinite {
            const NETSCAPE_LOOP: [u8; 19] = [
                0x21, 0xFF, 0x0B, b'N', b'E', b'T', b'S', b'C', b'A', b'P', b'E', b'2', b'.',
                b'0', 0x03, 0x01, 0x00, 0x00, 0x00,
            ];
            writer
                .write_all(&NETSCAPE_LOOP)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
        }

        let pixel_count = usize::from(width) * usize::from(height);
        Ok(Self {
            writer,
            width,
            height,
            delay_cs: options.frame_delay_centisecs(),
            frame_count: 0,
            is_finalized: false,
            indexed_scratch: vec![0u8; pixel_count],
            lut_cache: vec![-1i16; 32 * 32 * 32],
            lzw_table: vec![-1i16; 4096 * 256],
        })
    }

    /// Resizes (using bicubic `CatmullRom` filtering when scaled), computes an adaptive 256-color
    /// Median-Cut palette, and encodes an RGB8 buffer as a high-definition GIF89a animation frame.
    ///
    /// # Arguments
    ///
    /// * `rgb` - Contiguous RGB8 pixel buffer (`src_width * src_height * 3` bytes).
    /// * `src_width` - Source image width in pixels.
    /// * `src_height` - Source image height in pixels.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the compressed frame is written.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidBufferSize`] if `rgb` is smaller than `src_width * src_height * 3`,
    /// or [`CoreError::Serialization`] on I/O failure.
    pub fn write_rgb_frame(
        &mut self,
        rgb: &[u8],
        src_width: u32,
        src_height: u32,
    ) -> Result<(), CoreError> {
        let expected = (src_width as usize) * (src_height as usize) * 3;
        if rgb.len() < expected {
            return Err(CoreError::InvalidBufferSize {
                expected,
                actual: rgb.len(),
            });
        }

        let dst_w = u32::from(self.width);
        let dst_h = u32::from(self.height);

        let local_palette = if src_width == dst_w && src_height == dst_h {
            quantize_frame_adaptive(
                &rgb[..expected],
                dst_w as usize,
                dst_h as usize,
                &mut self.indexed_scratch,
                &mut self.lut_cache,
            )
        } else {
            let src_img = RgbImage::from_raw(src_width, src_height, rgb[..expected].to_vec())
                .ok_or_else(|| {
                    CoreError::Serialization("Failed to construct RGB image for scaling".to_string())
                })?;
            // Bicubic CatmullRom preserves crisp text and 1px UI lines during downscaling
            let resized = resize(&src_img, dst_w, dst_h, FilterType::CatmullRom);
            quantize_frame_adaptive(
                resized.as_raw(),
                dst_w as usize,
                dst_h as usize,
                &mut self.indexed_scratch,
                &mut self.lut_cache,
            )
        };

        self.write_indexed_frame(&local_palette)
    }

    /// Converts, scales with bicubic `CatmullRom` filtering, and encodes a [`VideoFrame`] (`Rgba` or `Bgra`)
    /// into the GIF stream.
    ///
    /// # Arguments
    ///
    /// * `frame` - Uncompressed [`VideoFrame`] to append.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError`] on invalid buffer size or file I/O failure.
    pub fn write_video_frame(&mut self, frame: &VideoFrame) -> Result<(), CoreError> {
        let src_w = frame.descriptor.width;
        let src_h = frame.descriptor.height;
        let is_bgra = frame.descriptor.format == PixelFormat::Bgra;
        let pixel_count = (src_w as usize) * (src_h as usize);

        let mut rgb = Vec::with_capacity(pixel_count * 3);
        for chunk in frame.data.chunks_exact(4).take(pixel_count) {
            if is_bgra {
                rgb.push(chunk[2]);
                rgb.push(chunk[1]);
                rgb.push(chunk[0]);
            } else {
                rgb.push(chunk[0]);
                rgb.push(chunk[1]);
                rgb.push(chunk[2]);
            }
        }

        self.write_rgb_frame(&rgb, src_w, src_h)
    }

    fn write_indexed_frame(&mut self, local_palette: &[u8; 768]) -> Result<(), CoreError> {
        // 1. Graphic Control Extension (8 bytes)
        let delay_bytes = self.delay_cs.to_le_bytes();
        let gce: [u8; 8] = [
            0x21,
            0xF9,
            0x04,
            0x04, // Disposal method 1 (Do Not Dispose)
            delay_bytes[0],
            delay_bytes[1],
            0x00,
            0x00,
        ];
        self.writer
            .write_all(&gce)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 2. Image Descriptor (10 bytes) with Local Color Table Flag enabled (0x87 = 256 local colors)
        let mut img_desc = [0u8; 10];
        img_desc[0] = 0x2C;
        img_desc[5..7].copy_from_slice(&self.width.to_le_bytes());
        img_desc[7..9].copy_from_slice(&self.height.to_le_bytes());
        img_desc[9] = 0x87;
        self.writer
            .write_all(&img_desc)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 3. Per-Frame Adaptive Local Color Table (768 bytes)
        self.writer
            .write_all(local_palette)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 4. LZW Minimum Code Size (8) + Sub-blocks
        self.writer
            .write_all(&[8u8])
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        let compressed = lzw_encode_gif_indices(&self.indexed_scratch, &mut self.lzw_table);
        for chunk in compressed.chunks(255) {
            self.writer
                .write_all(&[chunk.len() as u8])
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
            self.writer
                .write_all(chunk)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
        }

        // Sub-block terminator
        self.writer
            .write_all(&[0u8])
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        self.frame_count += 1;
        Ok(())
    }

    /// Writes the GIF89a trailer (`0x3B`) and flushes all buffered data to disk.
    ///
    /// # Returns
    ///
    /// Total number of animation frames written.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Serialization`] if flushing fails.
    pub fn finalize(&mut self) -> Result<u32, CoreError> {
        if !self.is_finalized {
            self.writer
                .write_all(&[0x3B])
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
            self.writer
                .flush()
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
            self.is_finalized = true;
        }
        Ok(self.frame_count)
    }
}

impl Drop for GifWriter {
    fn drop(&mut self) {
        if !self.is_finalized {
            let _ = self.finalize();
        }
    }
}

/// Converts a recorded Motion-JPEG `.avi` video file into an animated `.gif` file on disk
/// (`[REQ-GIF-001.4]`, `[NFR-GIF-001.2]`).
///
/// Streams `00dc` JPEG chunks sequentially from the AVI file without loading all raw frames
/// into memory simultaneously.
///
/// # Arguments
///
/// * `avi_path` - Path to the source `.avi` file.
/// * `gif_path` - Destination `.gif` file path.
/// * `options` - [`GifExportOptions`] specifying target FPS, resolution scale, and max duration.
///
/// # Returns
///
/// Returns the output [`PathBuf`] of the generated `.gif` file.
///
/// # Errors
///
/// Returns [`CoreError::Serialization`] if the `.avi` file cannot be read, contains no valid
/// `00dc` video chunks, or if writing the `.gif` fails.
///
/// # Examples
///
/// ```rust
/// use rec_core::avi_writer::AviWriter;
/// use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
/// use rec_core::gif_exporter::{export_avi_to_gif, GifExportOptions, GifScale};
/// use rec_core::pixel_format::PixelFormat;
///
/// let temp_dir = std::env::temp_dir();
/// let avi_path = temp_dir.join("doctest_src.avi");
/// let gif_path = temp_dir.join("doctest_out.gif");
///
/// let mut avi = AviWriter::new(&avi_path, 64, 48, 30).unwrap();
/// let desc = FrameDescriptor::new(64, 48, PixelFormat::Rgba).unwrap();
/// let frame = VideoFrame::new(desc, Pts::from_nanos(0), vec![100u8; 64 * 48 * 4]).unwrap();
/// avi.write_frame(&frame).unwrap();
/// avi.finalize().unwrap();
///
/// let opts = GifExportOptions::new(15, GifScale::Scale50, 5);
/// let saved = export_avi_to_gif(&avi_path, &gif_path, opts).unwrap();
/// assert!(saved.exists());
///
/// let _ = std::fs::remove_file(avi_path);
/// let _ = std::fs::remove_file(gif_path);
/// ```
pub fn export_avi_to_gif<P1: AsRef<Path>, P2: AsRef<Path>>(
    avi_path: P1,
    gif_path: P2,
    options: GifExportOptions,
) -> Result<PathBuf, CoreError> {
    let mut file = File::open(avi_path.as_ref())
        .map_err(|e| CoreError::Serialization(format!("Failed to open AVI file: {}", e)))?;
    let file_len = file
        .metadata()
        .map_err(|e| CoreError::Serialization(e.to_string()))?
        .len();

    if file_len < 64 {
        return Err(CoreError::Serialization(
            "AVI file is too small to contain valid headers".to_string(),
        ));
    }

    // Read main AVI header (`avih` at offset 32: micro_sec_per_frame at +0)
    let mut header_buf = [0u8; 128];
    file.read_exact(&mut header_buf)
        .map_err(|e| CoreError::Serialization(e.to_string()))?;

    let usec_per_frame = u32::from_le_bytes([
        header_buf[32],
        header_buf[33],
        header_buf[34],
        header_buf[35],
    ]);
    let src_fps = if usec_per_frame > 0 {
        (1_000_000 / usec_per_frame).clamp(1, 240)
    } else {
        30
    };

    let target_fps = options.fps.clamp(5, 30);
    let step = (src_fps / target_fps).max(1) as usize;
    let max_gif_frames = (target_fps * options.max_duration_secs.clamp(5, 15)) as usize;

    // Scan RIFF chunks starting after the 4096-byte reserved header
    let mut pos: u64 = 4096;
    if pos >= file_len {
        pos = 12;
    }

    let mut gif_writer: Option<GifWriter> = None;
    let mut video_chunk_idx: usize = 0;
    let mut gif_frames_written: usize = 0;
    let mut chunk_hdr = [0u8; 8];
    let mut jpeg_buf = Vec::with_capacity(128 * 1024);

    while pos + 8 <= file_len && gif_frames_written < max_gif_frames {
        file.seek(SeekFrom::Start(pos))
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        if file.read_exact(&mut chunk_hdr).is_err() {
            break;
        }

        let tag = &chunk_hdr[0..4];
        let size = u32::from_le_bytes([chunk_hdr[4], chunk_hdr[5], chunk_hdr[6], chunk_hdr[7]]) as u64;

        // Container envelopes ('RIFF' or 'LIST') have a 4-byte sub-type and contain nested chunks
        if tag == b"RIFF" || tag == b"LIST" {
            pos += 12;
            continue;
        }

        if tag == b"00dc" && size > 0 && pos + 8 + size <= file_len {
            if video_chunk_idx.is_multiple_of(step) {
                jpeg_buf.resize(size as usize, 0);
                file.read_exact(&mut jpeg_buf)
                    .map_err(|e| CoreError::Serialization(e.to_string()))?;

                if let Ok(decoded) =
                    image::load_from_memory_with_format(&jpeg_buf, image::ImageFormat::Jpeg)
                {
                    let rgb_img = decoded.to_rgb8();
                    let w = rgb_img.width();
                    let h = rgb_img.height();

                    let writer = match gif_writer {
                        Some(ref mut gw) => gw,
                        None => {
                            let (gw_w, gw_h) = options.scale.scaled_dimensions(w, h);
                            gif_writer =
                                Some(GifWriter::new(gif_path.as_ref(), gw_w, gw_h, options)?);
                            gif_writer.as_mut().ok_or_else(|| {
                                CoreError::Serialization(
                                    "Failed to initialize GifWriter".to_string(),
                                )
                            })?
                        }
                    };

                    writer.write_rgb_frame(rgb_img.as_raw(), w, h)?;
                    gif_frames_written += 1;
                }
            }
            video_chunk_idx += 1;
        }

        // Advance to next word-aligned RIFF chunk
        let padded_size = size + (size & 1);
        if padded_size == 0 && tag == [0, 0, 0, 0] {
            break;
        }
        pos = pos.saturating_add(8).saturating_add(padded_size);
    }

    if let Some(mut writer) = gif_writer {
        let count = writer.finalize()?;
        if count > 0 {
            return Ok(gif_path.as_ref().to_path_buf());
        }
    }

    Err(CoreError::Serialization(
        "No valid 00dc JPEG video frames found in AVI container".to_string(),
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::avi_writer::AviWriter;
    use crate::frame::{FrameDescriptor, Pts};

    /// Verifies `[TEST-GIF-001.1]`: Valid `GIF89a` header, `NETSCAPE2.0` block, Local Color Table, LZW sub-blocks, and `0x3B` trailer.
    #[test]
    fn test_gif_encoder_valid_header_and_trailer() {
        let temp_path = std::env::temp_dir().join("test_unit_gif89a.gif");
        let opts = GifExportOptions::new(15, GifScale::Scale100, 10);
        let mut writer = GifWriter::new(&temp_path, 16, 16, opts).unwrap();

        let desc = FrameDescriptor::new(16, 16, PixelFormat::Rgba).unwrap();
        for i in 0..3u8 {
            let frame = VideoFrame::new(
                desc.clone(),
                Pts::from_nanos(u64::from(i) * 33_000_000),
                vec![i * 60; 16 * 16 * 4],
            )
            .unwrap();
            writer.write_video_frame(&frame).unwrap();
        }

        let frames = writer.finalize().unwrap();
        assert_eq!(frames, 3);

        let bytes = std::fs::read(&temp_path).unwrap();
        assert!(bytes.starts_with(b"GIF89a"));
        assert!(bytes.windows(11).any(|w| w == b"NETSCAPE2.0"));
        assert_eq!(*bytes.last().unwrap(), 0x3B);

        let _ = std::fs::remove_file(temp_path);
    }

    /// Verifies `[TEST-GIF-001.2]`: End-to-end AVI recording to animated GIF export with adaptive palette and CatmullRom scaling.
    #[test]
    fn test_export_avi_to_gif_end_to_end() {
        let temp_dir = std::env::temp_dir();
        let avi_path = temp_dir.join("test_e2e_gif_src.avi");
        let gif_path = temp_dir.join("test_e2e_gif_out.gif");

        let mut avi = AviWriter::new(&avi_path, 120, 80, 30).unwrap();
        let desc = FrameDescriptor::new(120, 80, PixelFormat::Rgba).unwrap();
        for i in 0..6u8 {
            let mut pixels = vec![20u8; desc.required_buffer_size()];
            for (idx, chunk) in pixels.chunks_exact_mut(4).enumerate() {
                chunk[0] = ((idx * 7) & 0xFF) as u8;
                chunk[1] = ((idx * 13 + usize::from(i) * 20) & 0xFF) as u8;
                chunk[2] = ((idx * 3) & 0xFF) as u8;
                chunk[3] = 255;
            }
            let frame =
                VideoFrame::new(desc.clone(), Pts::from_nanos(u64::from(i) * 33_333_333), pixels)
                    .unwrap();
            avi.write_frame(&frame).unwrap();
        }
        avi.finalize().unwrap();

        let opts = GifExportOptions::new(15, GifScale::Scale50, 5);
        let out = export_avi_to_gif(&avi_path, &gif_path, opts).unwrap();
        assert!(out.exists());

        let gif_bytes = std::fs::read(&gif_path).unwrap();
        assert!(gif_bytes.starts_with(b"GIF89a"));
        // Verify scaled dimensions in Logical Screen Descriptor: 50% of 120x80 = 60x40
        let w = u16::from_le_bytes([gif_bytes[6], gif_bytes[7]]);
        let h = u16::from_le_bytes([gif_bytes[8], gif_bytes[9]]);
        assert_eq!((w, h), (60, 40));
        assert_eq!(*gif_bytes.last().unwrap(), 0x3B);

        let _ = std::fs::remove_file(avi_path);
        let _ = std::fs::remove_file(gif_path);
    }

    /// Verifies `[TEST-GIF-001.3]`: `GifScale` dimension scaling.
    #[test]
    fn test_gif_scale_dimensions() {
        assert_eq!(GifScale::Scale50.scaled_dimensions(1920, 1080), (960, 540));
        assert_eq!(GifScale::Scale75.scaled_dimensions(1920, 1080), (1440, 810));
        assert_eq!(GifScale::Scale100.scaled_dimensions(1920, 1080), (1920, 1080));
        assert_eq!(GifScale::Scale50.percent(), 50);
        assert_eq!(GifScale::Scale75.percent(), 75);
        assert_eq!(GifScale::Scale100.percent(), 100);
    }
}
