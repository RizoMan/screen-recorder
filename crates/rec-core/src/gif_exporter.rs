//! High-definition animated GIF89a exporter, perceptual palette quantizer, and AVI-to-GIF converter.
//!
//! Conforms to `[SPEC-FEAT-GIF-001]`, `[CONTRACT-GIF-001]`, `RULE-ARCH-001` (SOLID, DRY, KISS),
//! `RULE-SAFE-001` (`#![forbid(unsafe_code)]`, RAII), and `RULE-DOC-001`.

use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::errors::CoreError;
use crate::frame::VideoFrame;
use crate::pixel_format::PixelFormat;

/// 4×4 ordered Bayer dithering matrix normalized to signed offsets `[-7..=+8]`.
const BAYER_4X4: [[i16; 4]; 4] = [
    [-7, 1, -5, 3],
    [5, -3, 7, -1],
    [-4, 4, -6, 2],
    [8, 0, 6, -2],
];

/// Resolution scaling preset for animated GIF export (`[REQ-GIF-001.3]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GifScale {
    /// 50% width and height (compact file size for GitHub Issues / Slack).
    Scale50,
    /// 75% width and height (balanced HD clarity and size).
    #[default]
    Scale75,
    /// 100% original capture resolution.
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
            fps: 15,
            scale: GifScale::Scale75,
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
    /// * `fps` - Target GIF framerate (`10..=30`).
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

/// Builds the 256-color Global Color Table (`768` bytes):
/// - Indices `0..240`: `6` Red × `8` Green × `5` Blue perceptual color cube.
/// - Indices `240..256`: `16`-step precision neutral grayscale ramp for crisp dark/light UI surfaces.
fn build_global_palette() -> [u8; 768] {
    let mut palette = [0u8; 768];
    let mut idx = 0;

    for r in 0..6u16 {
        let rv = ((r * 255) / 5) as u8;
        for g in 0..8u16 {
            let gv = ((g * 255) / 7) as u8;
            for b in 0..5u16 {
                let bv = ((b * 255) / 4) as u8;
                palette[idx] = rv;
                palette[idx + 1] = gv;
                palette[idx + 2] = bv;
                idx += 3;
            }
        }
    }

    for gray in 0..16u16 {
        let v = (gray * 17) as u8;
        palette[idx] = v;
        palette[idx + 1] = v;
        palette[idx + 2] = v;
        idx += 3;
    }

    palette
}

/// Quantizes an RGB pixel `(r, g, b)` at coordinates `(x, y)` into a 256-color palette index
/// using neutral grayscale detection and 4×4 Bayer ordered dithering (`[REQ-GIF-001.2]`).
#[inline]
fn quantize_pixel(r: u8, g: u8, b: u8, x: usize, y: usize) -> u8 {
    // Preserve solid dark/light UI neutrals on the 16-level grayscale ramp without dither noise
    if r.abs_diff(g) <= 6 && g.abs_diff(b) <= 6 {
        let lum = (u16::from(r) + u16::from(g) * 2 + u16::from(b)) / 4;
        let gray_idx = ((lum * 15 + 127) / 255) as u8;
        return 240 + gray_idx.min(15);
    }

    let dither = BAYER_4X4[y & 3][x & 3];
    let dr = (i16::from(r) + dither).clamp(0, 255) as u16;
    let dg = (i16::from(g) + dither).clamp(0, 255) as u16;
    let db = (i16::from(b) + dither).clamp(0, 255) as u16;

    let r_idx = ((dr * 5 + 127) / 255).min(5) as u8;
    let g_idx = ((dg * 7 + 127) / 255).min(7) as u8;
    let b_idx = ((db * 4 + 127) / 255).min(4) as u8;

    r_idx * 40 + g_idx * 5 + b_idx
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

/// Pure safe Rust GIF89a animated image writer with RAII finalization (`[REQ-GIF-001.1]`).
pub struct GifWriter {
    writer: BufWriter<File>,
    width: u16,
    height: u16,
    delay_cs: u16,
    frame_count: u32,
    is_finalized: bool,
    indexed_scratch: Vec<u8>,
    lzw_table: Vec<i16>,
}

impl GifWriter {
    /// Creates a new [`GifWriter`] on disk and writes the `GIF89a` header, Logical Screen Descriptor,
    /// 256-color Global Color Table, and optional `NETSCAPE2.0` loop extension.
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
        let mut writer = BufWriter::with_capacity(128 * 1024, file);

        // 1. GIF89a Header
        writer
            .write_all(b"GIF89a")
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 2. Logical Screen Descriptor (Global Color Table = 256 entries -> 0xF7)
        let mut lsd = [0u8; 7];
        lsd[0..2].copy_from_slice(&width.to_le_bytes());
        lsd[2..4].copy_from_slice(&height.to_le_bytes());
        lsd[4] = 0xF7;
        lsd[5] = 0;
        lsd[6] = 0;
        writer
            .write_all(&lsd)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 3. Global Color Table (768 bytes)
        let palette = build_global_palette();
        writer
            .write_all(&palette)
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
            lzw_table: vec![-1i16; 4096 * 256],
        })
    }

    /// Quantizes, resizes (nearest/box sampling if dimensions differ), and encodes a raw RGB8 buffer
    /// as a GIF89a animation frame.
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

        let dst_w = usize::from(self.width);
        let dst_h = usize::from(self.height);
        let src_w = src_width as usize;
        let src_h = src_height as usize;

        for y in 0..dst_h {
            let sy = ((y * src_h) / dst_h).min(src_h.saturating_sub(1));
            let row_offset = sy * src_w * 3;
            let dst_row_offset = y * dst_w;
            for x in 0..dst_w {
                let sx = ((x * src_w) / dst_w).min(src_w.saturating_sub(1));
                let px = row_offset + sx * 3;
                let r = rgb[px];
                let g = rgb[px + 1];
                let b = rgb[px + 2];
                self.indexed_scratch[dst_row_offset + x] = quantize_pixel(r, g, b, x, y);
            }
        }

        self.write_indexed_frame()
    }

    /// Quantizes, scales, and encodes a [`VideoFrame`] (`Rgba` or `Bgra`) into the GIF stream.
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
        let dst_w = usize::from(self.width);
        let dst_h = usize::from(self.height);
        let src_w = frame.descriptor.width as usize;
        let src_h = frame.descriptor.height as usize;
        let is_bgra = frame.descriptor.format == PixelFormat::Bgra;

        for y in 0..dst_h {
            let sy = ((y * src_h) / dst_h).min(src_h.saturating_sub(1));
            let row_offset = sy * src_w * 4;
            let dst_row_offset = y * dst_w;
            for x in 0..dst_w {
                let sx = ((x * src_w) / dst_w).min(src_w.saturating_sub(1));
                let px = row_offset + sx * 4;
                let (r, g, b) = if is_bgra {
                    (frame.data[px + 2], frame.data[px + 1], frame.data[px])
                } else {
                    (frame.data[px], frame.data[px + 1], frame.data[px + 2])
                };
                self.indexed_scratch[dst_row_offset + x] = quantize_pixel(r, g, b, x, y);
            }
        }

        self.write_indexed_frame()
    }

    fn write_indexed_frame(&mut self) -> Result<(), CoreError> {
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

        // 2. Image Descriptor (10 bytes)
        let mut img_desc = [0u8; 10];
        img_desc[0] = 0x2C;
        img_desc[5..7].copy_from_slice(&self.width.to_le_bytes());
        img_desc[7..9].copy_from_slice(&self.height.to_le_bytes());
        self.writer
            .write_all(&img_desc)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        // 3. LZW Minimum Code Size (8) + Sub-blocks
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

    // Read main AVI header (`avih` at offset 32: micro_sec_per_frame at +0, width at +32, height at +36)
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
                            gif_writer = Some(GifWriter::new(gif_path.as_ref(), gw_w, gw_h, options)?);
                            gif_writer.as_mut().ok_or_else(|| {
                                CoreError::Serialization("Failed to initialize GifWriter".to_string())
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

    /// Verifies `[TEST-GIF-001.1]`: Valid `GIF89a` header, `NETSCAPE2.0` block, LZW sub-blocks, and `0x3B` trailer.
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

    /// Verifies `[TEST-GIF-001.2]`: End-to-end AVI recording to animated GIF export.
    #[test]
    fn test_export_avi_to_gif_end_to_end() {
        let temp_dir = std::env::temp_dir();
        let avi_path = temp_dir.join("test_e2e_gif_src.avi");
        let gif_path = temp_dir.join("test_e2e_gif_out.gif");

        let mut avi = AviWriter::new(&avi_path, 120, 80, 30).unwrap();
        let desc = FrameDescriptor::new(120, 80, PixelFormat::Rgba).unwrap();
        for i in 0..6u8 {
            let mut pixels = vec![20u8; desc.required_buffer_size()];
            // Add a colored region so both neutral and chromatic quantization paths are exercised
            for chunk in pixels.chunks_exact_mut(4).take(200) {
                chunk[0] = 240;
                chunk[1] = 120 + i * 10;
                chunk[2] = 30;
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
