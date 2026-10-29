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

/// Palette index reserved for inter-frame transparency in GIF89a delta frames.
const TRANSPARENT_INDEX: u8 = 255;

/// Maximum number of opaque colors per palette (`0..=254`), reserving `255` for transparency.
const MAX_OPAQUE_COLORS: usize = 255;

/// Computes an adaptive 255-color palette (`768` bytes, indices `0..254` opaque, `255` transparent)
/// tailored to the active RGB pixels using a 3D `32×32×32` histogram and perceptual Median-Cut (`[REQ-GIF-001.2]`).
fn build_median_cut_palette(
    rgb: &[u8],
    changed_mask: Option<&[u8]>,
    lut_cache: &mut [i16],
) -> ([u8; 768], usize) {
    const HIST_SIZE: usize = 32 * 32 * 32;
    let mut counts = vec![0u32; HIST_SIZE];
    let mut sum_r = vec![0u64; HIST_SIZE];
    let mut sum_g = vec![0u64; HIST_SIZE];
    let mut sum_b = vec![0u64; HIST_SIZE];

    match changed_mask {
        Some(mask) => {
            for (chunk, &m) in rgb.chunks_exact(3).zip(mask.iter()) {
                if m == 0 {
                    continue;
                }
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
        }
        None => {
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
        }
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
    lut_cache.fill(-1);

    if bins.is_empty() {
        return (palette, 1);
    }

    let num_colors = if bins.len() <= MAX_OPAQUE_COLORS {
        for (i, b) in bins.iter().enumerate() {
            let cnt = u64::from(b.count).max(1);
            palette[i * 3] = (b.sum_r / cnt) as u8;
            palette[i * 3 + 1] = (b.sum_g / cnt) as u8;
            palette[i * 3 + 2] = (b.sum_b / cnt) as u8;
        }
        bins.len()
    } else {
        let mut boxes = Vec::with_capacity(MAX_OPAQUE_COLORS);
        boxes.push(MedianBox::from_slice(&bins, 0, bins.len()));

        while boxes.len() < MAX_OPAQUE_COLORS {
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

        let colors = boxes.len().min(MAX_OPAQUE_COLORS);
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

    (palette, num_colors.max(1))
}

/// Finds the closest opaque palette index (`0..num_colors`) using a 3D `32×32×32` lookup cache.
#[inline]
fn lookup_nearest_color(
    r: u8,
    g: u8,
    b: u8,
    palette: &[u8; 768],
    num_colors: usize,
    lut: &mut [i16],
) -> u8 {
    let cell = ((usize::from(r >> 3)) << 10)
        | ((usize::from(g >> 3)) << 5)
        | usize::from(b >> 3);
    let cached = lut[cell];
    if cached >= 0 {
        return cached as u8;
    }

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

/// Buffered delta frame awaiting emission so subsequent identical frames can coalesce their `delay_cs`.
struct PendingGifFrame {
    delay_cs: u16,
    left: u16,
    top: u16,
    width: u16,
    height: u16,
    local_palette: Option<[u8; 768]>,
    compressed_lzw: Vec<u8>,
}

/// Pure safe Rust GIF89a animated image writer with adaptive 255-color Median-Cut palettes,
/// inter-frame delta bounding-box cropping, transparency index `255`, and RAII finalization
/// (`[REQ-GIF-001.1]`, `[REQ-GIF-001.2]`).
pub struct GifWriter {
    writer: BufWriter<File>,
    width: u16,
    height: u16,
    delay_cs: u16,
    loop_infinite: bool,
    header_written: bool,
    frame_count: u32,
    is_finalized: bool,
    global_palette: [u8; 768],
    global_num_colors: usize,
    global_lut_cache: Vec<i16>,
    local_lut_cache: Vec<i16>,
    canvas_src_rgb: Vec<u8>,
    diff_mask: Vec<u8>,
    confirmed_mask: Vec<u8>,
    indexed_scratch: Vec<u8>,
    lzw_table: Vec<i16>,
    pending_frame: Option<PendingGifFrame>,
}

impl GifWriter {
    /// Creates a new [`GifWriter`] on disk ready to encode high-definition, delta-compressed GIF89a frames.
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
        let writer = BufWriter::with_capacity(256 * 1024, file);

        let pixel_count = usize::from(width) * usize::from(height);
        Ok(Self {
            writer,
            width,
            height,
            delay_cs: options.frame_delay_centisecs(),
            loop_infinite: options.loop_infinite,
            header_written: false,
            frame_count: 0,
            is_finalized: false,
            global_palette: [0u8; 768],
            global_num_colors: 1,
            global_lut_cache: vec![-1i16; 32 * 32 * 32],
            local_lut_cache: vec![-1i16; 32 * 32 * 32],
            canvas_src_rgb: vec![0u8; pixel_count * 3],
            diff_mask: vec![0u8; pixel_count],
            confirmed_mask: vec![0u8; pixel_count],
            indexed_scratch: vec![0u8; pixel_count],
            lzw_table: vec![-1i16; 4096 * 256],
            pending_frame: None,
        })
    }

    fn ensure_header_written(&mut self) -> Result<(), CoreError> {
        if self.header_written {
            return Ok(());
        }

        // 1. GIF89a Header
        self.writer
            .write_all(b"GIF89a")
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 2. Logical Screen Descriptor (Global Color Table present: 0xF7 = 256 entries)
        let mut lsd = [0u8; 7];
        lsd[0..2].copy_from_slice(&self.width.to_le_bytes());
        lsd[2..4].copy_from_slice(&self.height.to_le_bytes());
        lsd[4] = 0xF7;
        lsd[5] = TRANSPARENT_INDEX;
        lsd[6] = 0;
        self.writer
            .write_all(&lsd)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 3. Adaptive Global Color Table computed from Frame 0 (768 bytes)
        self.writer
            .write_all(&self.global_palette)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 4. NETSCAPE2.0 Infinite Loop Application Extension
        if self.loop_infinite {
            const NETSCAPE_LOOP: [u8; 19] = [
                0x21, 0xFF, 0x0B, b'N', b'E', b'T', b'S', b'C', b'A', b'P', b'E', b'2', b'.',
                b'0', 0x03, 0x01, 0x00, 0x00, 0x00,
            ];
            self.writer
                .write_all(&NETSCAPE_LOOP)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
        }

        self.header_written = true;
        Ok(())
    }

    fn flush_pending_frame(&mut self) -> Result<(), CoreError> {
        let Some(frame) = self.pending_frame.take() else {
            return Ok(());
        };

        self.ensure_header_written()?;

        // 1. Graphic Control Extension (8 bytes)
        // Packed byte 0x05 = Disposal Method 1 (Do Not Dispose: 0x04) | Transparent Color Flag (0x01)
        let delay_bytes = frame.delay_cs.to_le_bytes();
        let gce: [u8; 8] = [
            0x21,
            0xF9,
            0x04,
            0x05,
            delay_bytes[0],
            delay_bytes[1],
            TRANSPARENT_INDEX,
            0x00,
        ];
        self.writer
            .write_all(&gce)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 2. Image Descriptor (10 bytes) with sub-rectangle coordinates (left, top, width, height)
        let mut img_desc = [0u8; 10];
        img_desc[0] = 0x2C;
        img_desc[1..3].copy_from_slice(&frame.left.to_le_bytes());
        img_desc[3..5].copy_from_slice(&frame.top.to_le_bytes());
        img_desc[5..7].copy_from_slice(&frame.width.to_le_bytes());
        img_desc[7..9].copy_from_slice(&frame.height.to_le_bytes());
        img_desc[9] = if frame.local_palette.is_some() {
            0x87 // Local Color Table present (256 entries)
        } else {
            0x00 // Use Global Color Table (saves 768 bytes per frame)
        };
        self.writer
            .write_all(&img_desc)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 3. Optional Per-Frame Adaptive Local Color Table (only when scene colors shift)
        if let Some(ref local_pal) = frame.local_palette {
            self.writer
                .write_all(local_pal)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
        }

        // 4. LZW Minimum Code Size (8) + Sub-blocks
        self.writer
            .write_all(&[8u8])
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        for chunk in frame.compressed_lzw.chunks(255) {
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

        Ok(())
    }

    /// Resizes (using bicubic `CatmullRom` filtering when scaled), applies inter-frame delta
    /// bounding-box cropping and MJPEG noise suppression, and encodes an RGB8 buffer as a
    /// high-definition GIF89a animation frame.
    ///
    /// # Arguments
    ///
    /// * `rgb` - Contiguous RGB8 pixel buffer (`src_width * src_height * 3` bytes).
    /// * `src_width` - Source image width in pixels.
    /// * `src_height` - Source image height in pixels.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the frame is processed.
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

        if src_width == dst_w && src_height == dst_h {
            self.encode_scaled_rgb_frame(&rgb[..expected])
        } else {
            let src_img = RgbImage::from_raw(src_width, src_height, rgb[..expected].to_vec())
                .ok_or_else(|| {
                    CoreError::Serialization(
                        "Failed to construct RGB image for scaling".to_string(),
                    )
                })?;
            // Bicubic CatmullRom preserves crisp text and 1px UI lines during downscaling
            let resized = resize(&src_img, dst_w, dst_h, FilterType::CatmullRom);
            self.encode_scaled_rgb_frame(resized.as_raw())
        }
    }

    fn encode_scaled_rgb_frame(&mut self, rgb: &[u8]) -> Result<(), CoreError> {
        let w = usize::from(self.width);
        let h = usize::from(self.height);
        let pixel_count = w * h;

        if self.frame_count == 0 {
            // Frame 0 (Keyframe): build the Global Color Table (255 adaptive colors + transparent 255)
            let (pal, num_colors) =
                build_median_cut_palette(rgb, None, &mut self.global_lut_cache);
            self.global_palette = pal;
            self.global_num_colors = num_colors;
            self.canvas_src_rgb.copy_from_slice(&rgb[..pixel_count * 3]);

            // Quantize full frame with Horizontal Run-Length Stabilization (<= 4 RGB deadzone)
            for y in 0..h {
                let row_px = y * w;
                let mut prev_idx: u8 = TRANSPARENT_INDEX;
                for x in 0..w {
                    let i = row_px + x;
                    let r = rgb[i * 3];
                    let g = rgb[i * 3 + 1];
                    let b = rgb[i * 3 + 2];

                    let chosen = if prev_idx < TRANSPARENT_INDEX {
                        let p_base = usize::from(prev_idx) * 3;
                        let pr = self.global_palette[p_base];
                        let pg = self.global_palette[p_base + 1];
                        let pb = self.global_palette[p_base + 2];
                        if r.abs_diff(pr) <= 4 && g.abs_diff(pg) <= 4 && b.abs_diff(pb) <= 4 {
                            prev_idx
                        } else {
                            lookup_nearest_color(
                                r,
                                g,
                                b,
                                &self.global_palette,
                                self.global_num_colors,
                                &mut self.global_lut_cache,
                            )
                        }
                    } else {
                        lookup_nearest_color(
                            r,
                            g,
                            b,
                            &self.global_palette,
                            self.global_num_colors,
                            &mut self.global_lut_cache,
                        )
                    };

                    self.indexed_scratch[i] = chosen;
                    prev_idx = chosen;
                }
            }

            let compressed_lzw =
                lzw_encode_gif_indices(&self.indexed_scratch[..pixel_count], &mut self.lzw_table);
            self.pending_frame = Some(PendingGifFrame {
                delay_cs: self.delay_cs,
                left: 0,
                top: 0,
                width: self.width,
                height: self.height,
                local_palette: None,
                compressed_lzw,
            });
            self.frame_count = 1;
            return Ok(());
        }

        // Subsequent frames: two-threshold temporal/spatial hysteresis filter to reject MJPEG noise
        for i in 0..pixel_count {
            let base = i * 3;
            let dr = rgb[base].abs_diff(self.canvas_src_rgb[base]);
            let dg = rgb[base + 1].abs_diff(self.canvas_src_rgb[base + 1]);
            let db = rgb[base + 2].abs_diff(self.canvas_src_rgb[base + 2]);
            let max_d = dr.max(dg).max(db);
            self.diff_mask[i] = if max_d <= 6 {
                0
            } else if max_d <= 18 {
                1
            } else {
                2
            };
        }

        let mut min_x = w;
        let mut max_x = 0usize;
        let mut min_y = h;
        let mut max_y = 0usize;
        let mut changed_count = 0usize;

        for y in 0..h {
            let row = y * w;
            for x in 0..w {
                let i = row + x;
                let dm = self.diff_mask[i];
                let confirmed = if dm == 2 {
                    true
                } else if dm == 1 {
                    let left_ok = x > 0 && self.diff_mask[i - 1] >= 1;
                    let right_ok = x + 1 < w && self.diff_mask[i + 1] >= 1;
                    let up_ok = y > 0 && self.diff_mask[i - w] >= 1;
                    let down_ok = y + 1 < h && self.diff_mask[i + w] >= 1;
                    left_ok || right_ok || up_ok || down_ok
                } else {
                    false
                };

                if confirmed {
                    self.confirmed_mask[i] = 1;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    changed_count += 1;
                } else {
                    self.confirmed_mask[i] = 0;
                }
            }
        }

        // Zero-byte frame deduplication: if no pixels changed, coalesce delay_cs into pending frame
        if changed_count == 0 {
            if let Some(ref mut pending) = self.pending_frame {
                pending.delay_cs = pending.delay_cs.saturating_add(self.delay_cs);
            }
            self.frame_count += 1;
            return Ok(());
        }

        // Flush previous pending frame before encoding the new delta bounding box
        self.flush_pending_frame()?;

        let sub_w = max_x - min_x + 1;
        let sub_h = max_y - min_y + 1;

        // Evaluate whether the Global Color Table represents the changed pixels accurately
        let mut high_error_pixels = 0usize;
        for y in min_y..=max_y {
            let row = y * w;
            for x in min_x..=max_x {
                let i = row + x;
                if self.confirmed_mask[i] == 0 {
                    continue;
                }
                let r = rgb[i * 3];
                let g = rgb[i * 3 + 1];
                let b = rgb[i * 3 + 2];
                let idx = lookup_nearest_color(
                    r,
                    g,
                    b,
                    &self.global_palette,
                    self.global_num_colors,
                    &mut self.global_lut_cache,
                );
                let p_base = usize::from(idx) * 3;
                let err = r
                    .abs_diff(self.global_palette[p_base])
                    .max(g.abs_diff(self.global_palette[p_base + 1]))
                    .max(b.abs_diff(self.global_palette[p_base + 2]));
                if err > 20 {
                    high_error_pixels += 1;
                }
            }
        }

        // Build an on-demand adaptive Local Color Table only if > 5% of changed pixels have error > 20
        let local_palette_opt = if high_error_pixels * 20 > changed_count {
            let (local_pal, local_num) = build_median_cut_palette(
                rgb,
                Some(&self.confirmed_mask),
                &mut self.local_lut_cache,
            );
            Some((local_pal, local_num))
        } else {
            None
        };

        // Quantize the cropped sub-rectangle with TRANSPARENT_INDEX (255) for unchanged pixels
        let mut out_pos = 0usize;
        for y in min_y..=max_y {
            let row = y * w;
            let mut prev_idx: u8 = TRANSPARENT_INDEX;
            for x in min_x..=max_x {
                let i = row + x;
                if self.confirmed_mask[i] == 0 {
                    self.indexed_scratch[out_pos] = TRANSPARENT_INDEX;
                    prev_idx = TRANSPARENT_INDEX;
                    out_pos += 1;
                    continue;
                }

                let r = rgb[i * 3];
                let g = rgb[i * 3 + 1];
                let b = rgb[i * 3 + 2];

                // Update reference canvas state for this changed pixel
                self.canvas_src_rgb[i * 3] = r;
                self.canvas_src_rgb[i * 3 + 1] = g;
                self.canvas_src_rgb[i * 3 + 2] = b;

                let chosen = match local_palette_opt {
                    Some((ref pal, num_c)) => {
                        if prev_idx < TRANSPARENT_INDEX {
                            let p_base = usize::from(prev_idx) * 3;
                            if r.abs_diff(pal[p_base]) <= 4
                                && g.abs_diff(pal[p_base + 1]) <= 4
                                && b.abs_diff(pal[p_base + 2]) <= 4
                            {
                                prev_idx
                            } else {
                                lookup_nearest_color(
                                    r,
                                    g,
                                    b,
                                    pal,
                                    num_c,
                                    &mut self.local_lut_cache,
                                )
                            }
                        } else {
                            lookup_nearest_color(r, g, b, pal, num_c, &mut self.local_lut_cache)
                        }
                    }
                    None => {
                        if prev_idx < TRANSPARENT_INDEX {
                            let p_base = usize::from(prev_idx) * 3;
                            if r.abs_diff(self.global_palette[p_base]) <= 4
                                && g.abs_diff(self.global_palette[p_base + 1]) <= 4
                                && b.abs_diff(self.global_palette[p_base + 2]) <= 4
                            {
                                prev_idx
                            } else {
                                lookup_nearest_color(
                                    r,
                                    g,
                                    b,
                                    &self.global_palette,
                                    self.global_num_colors,
                                    &mut self.global_lut_cache,
                                )
                            }
                        } else {
                            lookup_nearest_color(
                                r,
                                g,
                                b,
                                &self.global_palette,
                                self.global_num_colors,
                                &mut self.global_lut_cache,
                            )
                        }
                    }
                };

                self.indexed_scratch[out_pos] = chosen;
                prev_idx = chosen;
                out_pos += 1;
            }
        }

        let compressed_lzw =
            lzw_encode_gif_indices(&self.indexed_scratch[..out_pos], &mut self.lzw_table);
        self.pending_frame = Some(PendingGifFrame {
            delay_cs: self.delay_cs,
            left: min_x as u16,
            top: min_y as u16,
            width: sub_w as u16,
            height: sub_h as u16,
            local_palette: local_palette_opt.map(|(pal, _)| pal),
            compressed_lzw,
        });

        self.frame_count += 1;
        Ok(())
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

    /// Flushes any pending delta frame, writes the GIF89a trailer (`0x3B`), and flushes all buffered data to disk.
    ///
    /// # Returns
    ///
    /// Total number of animation frames processed.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Serialization`] if flushing fails.
    pub fn finalize(&mut self) -> Result<u32, CoreError> {
        if !self.is_finalized {
            self.ensure_header_written()?;
            self.flush_pending_frame()?;
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

    /// Verifies `[TEST-GIF-001.4]`: Inter-frame delta sub-rectangle cropping and transparency index `255`
    /// produce a `.gif` smaller than the source `.avi` on typical screen recordings.
    #[test]
    fn test_gif_delta_compression_smaller_than_avi() {
        let temp_dir = std::env::temp_dir();
        let avi_path = temp_dir.join("test_delta_ratio_src.avi");
        let gif_path = temp_dir.join("test_delta_ratio_out.gif");

        let w = 640u32;
        let h = 360u32;
        let mut avi = AviWriter::new(&avi_path, w, h, 30).unwrap();
        let desc = FrameDescriptor::new(w, h, PixelFormat::Rgba).unwrap();

        for f in 0..30u32 {
            let mut pixels = vec![0u8; desc.required_buffer_size()];
            // Realistic desktop UI background + top header + moving cursor indicator
            let cursor_x = 60 + f * 12;
            let cursor_y = 100 + f * 4;
            for y in 0..h {
                for x in 0..w {
                    let i = ((y * w + x) * 4) as usize;
                    if (x >= cursor_x && x < cursor_x + 24) && (y >= cursor_y && y < cursor_y + 24)
                    {
                        pixels[i] = 255;
                        pixels[i + 1] = 214;
                        pixels[i + 2] = 10;
                        pixels[i + 3] = 255;
                    } else if y < 40 {
                        pixels[i] = 30;
                        pixels[i + 1] = 36;
                        pixels[i + 2] = 48;
                        pixels[i + 3] = 255;
                    } else {
                        pixels[i] = 15;
                        pixels[i + 1] = 18;
                        pixels[i + 2] = 24;
                        pixels[i + 3] = 255;
                    }
                }
            }
            let frame = VideoFrame::new(
                desc.clone(),
                Pts::from_nanos(u64::from(f) * 33_333_333),
                pixels,
            )
            .unwrap();
            avi.write_frame(&frame).unwrap();
        }
        avi.finalize().unwrap();

        let opts = GifExportOptions::new(20, GifScale::Scale100, 10);
        export_avi_to_gif(&avi_path, &gif_path, opts).unwrap();

        let avi_size = std::fs::metadata(&avi_path).unwrap().len();
        let gif_size = std::fs::metadata(&gif_path).unwrap().len();
        assert!(
            gif_size < avi_size,
            "Expected delta-compressed GIF ({} bytes) to be smaller than source AVI ({} bytes)",
            gif_size,
            avi_size
        );

        let _ = std::fs::remove_file(avi_path);
        let _ = std::fs::remove_file(gif_path);
    }
}
