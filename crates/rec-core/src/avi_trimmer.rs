//! Lossless post-recording Quick Trim editor and AVI container remuxer (`[SPEC-FEAT-TRIM-001]`).
//!
//! Performs zero-reencoding stream copy of `00dc` Motion-JPEG video chunks and synchronized
//! `01wb` 16-bit PCM audio slices into a new OpenDML 2.0 + AVI 1.0 indexed `.avi` container.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::avi_writer::AviWriter;
use crate::errors::CoreError;

/// Probed metadata of a recorded `.avi` video file (`[REQ-TRIM-001.1]`, `[CONTRACT-TRIM-001]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AviClipMetadata {
    /// Video width in pixels.
    pub width: u32,
    /// Video height in pixels.
    pub height: u32,
    /// Playback frame rate in frames per second.
    pub fps: u32,
    /// Total number of `00dc` video chunks in the container.
    pub total_frames: u32,
    /// Total clip duration in seconds.
    pub duration_secs: f64,
    /// Audio sampling rate in Hz (default `48_000` if no audio stream is present).
    pub audio_sample_rate: u32,
    /// Audio channel count (default `2` if no audio stream is present).
    pub audio_channels: u16,
}

/// Validated In/Out trim range in seconds (`[REQ-TRIM-001.2]`, `[CONTRACT-TRIM-001]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimRange {
    /// Trim start timestamp in seconds (**In Point**).
    pub start_sec: f64,
    /// Trim end timestamp in seconds (**Out Point**).
    pub end_sec: f64,
}

impl TrimRange {
    /// Minimum allowed trimmed clip duration in seconds (`200 ms`).
    pub const MIN_DURATION_SECS: f64 = 0.2;

    /// Constructs a validated and sanitized [`TrimRange`] clamped within `0.0..=max_duration_secs`.
    ///
    /// # Arguments
    ///
    /// * `start_sec` - Requested start time in seconds (**In Point**).
    /// * `end_sec` - Requested end time in seconds (**Out Point**).
    /// * `max_duration_secs` - Total duration of the source clip in seconds.
    ///
    /// # Returns
    ///
    /// Sanitized [`TrimRange`] guaranteeing `0.0 <= start_sec < end_sec <= max_duration_secs`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::avi_trimmer::TrimRange;
    ///
    /// let range = TrimRange::new(1.5, 8.0, 10.0);
    /// assert_eq!(range.start_sec, 1.5);
    /// assert_eq!(range.end_sec, 8.0);
    /// assert!((range.trimmed_duration_secs() - 6.5).abs() < 1e-6);
    /// ```
    #[must_use]
    pub fn new(start_sec: f64, end_sec: f64, max_duration_secs: f64) -> Self {
        let total = max_duration_secs.max(Self::MIN_DURATION_SECS);
        let s = start_sec.clamp(0.0, (total - Self::MIN_DURATION_SECS).max(0.0));
        let e = end_sec.clamp(s + Self::MIN_DURATION_SECS, total);
        Self {
            start_sec: s,
            end_sec: e,
        }
    }

    /// Returns the duration of the trimmed clip in seconds (`end_sec - start_sec`).
    ///
    /// # Returns
    ///
    /// Positive duration in seconds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::avi_trimmer::TrimRange;
    ///
    /// let range = TrimRange::new(2.0, 7.5, 10.0);
    /// assert!((range.trimmed_duration_secs() - 5.5).abs() < 1e-6);
    /// ```
    #[must_use]
    pub fn trimmed_duration_secs(&self) -> f64 {
        (self.end_sec - self.start_sec).max(0.0)
    }

    /// Computes the inclusive-start, exclusive-end video frame index range `(start_frame, end_frame)`.
    ///
    /// # Arguments
    ///
    /// * `fps` - Source video frame rate in frames per second.
    /// * `total_frames` - Total number of video frames in the source container.
    ///
    /// # Returns
    ///
    /// Tuple `(start_frame, end_frame)` with `start_frame < end_frame <= total_frames`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::avi_trimmer::TrimRange;
    ///
    /// let range = TrimRange::new(1.0, 3.0, 5.0);
    /// assert_eq!(range.frame_bounds(30, 150), (30, 90));
    /// ```
    #[must_use]
    pub fn frame_bounds(&self, fps: u32, total_frames: u32) -> (u32, u32) {
        let total = total_frames.max(1);
        let fps_f = f64::from(fps.max(1));
        let start_f = (self.start_sec * fps_f).floor().max(0.0) as u32;
        let end_f = (self.end_sec * fps_f).ceil().max(1.0) as u32;
        let clamped_start = start_f.min(total.saturating_sub(1));
        let clamped_end = end_f.clamp(clamped_start + 1, total);
        (clamped_start, clamped_end)
    }

    /// Computes the sample-aligned 16-bit PCM byte offset range `(start_byte, end_byte)`.
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Audio sample rate in Hz (e.g., `48_000`).
    /// * `channels` - Number of audio channels (e.g., `2`).
    ///
    /// # Returns
    ///
    /// Tuple `(start_byte, end_byte)` aligned to `channels * 2` bytes per sample frame.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::avi_trimmer::TrimRange;
    ///
    /// let range = TrimRange::new(1.0, 2.0, 5.0);
    /// assert_eq!(range.audio_byte_bounds(48_000, 2), (192_000, 384_000));
    /// ```
    #[must_use]
    pub fn audio_byte_bounds(&self, sample_rate: u32, channels: u16) -> (u64, u64) {
        let block_align = u64::from(channels.max(1)) * 2;
        let sr = f64::from(sample_rate.max(1));
        let start_samples = (self.start_sec * sr).round().max(0.0) as u64;
        let end_samples = (self.end_sec * sr).round().max(start_samples as f64) as u64;
        (start_samples * block_align, end_samples * block_align)
    }
}

/// Probes an `.avi` video file on disk to extract dimensions, frame rate, frame count, and duration
/// without decoding JPEG frames (`[REQ-TRIM-001.1]`).
///
/// # Arguments
///
/// * `avi_path` - Path to the `.avi` file to inspect.
///
/// # Returns
///
/// Populated [`AviClipMetadata`].
///
/// # Errors
///
/// Returns [`CoreError::Serialization`] if the file cannot be read or is not a valid RIFF AVI container.
///
/// # Examples
///
/// ```rust
/// use rec_core::avi_trimmer::probe_avi_metadata;
/// use rec_core::avi_writer::AviWriter;
/// use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
/// use rec_core::pixel_format::PixelFormat;
///
/// let temp_path = std::env::temp_dir().join("doctest_probe_meta.avi");
/// let mut writer = AviWriter::new(&temp_path, 320, 240, 30).unwrap();
/// let desc = FrameDescriptor::new(320, 240, PixelFormat::Rgba).unwrap();
/// for i in 0..15u64 {
///     let frame = VideoFrame::new(desc.clone(), Pts::from_nanos(i * 33_333_333), vec![64u8; 320 * 240 * 4]).unwrap();
///     writer.write_frame(&frame).unwrap();
/// }
/// writer.finalize().unwrap();
///
/// let meta = probe_avi_metadata(&temp_path).unwrap();
/// assert_eq!(meta.width, 320);
/// assert_eq!(meta.height, 240);
/// assert_eq!(meta.fps, 30);
/// assert_eq!(meta.total_frames, 15);
/// let _ = std::fs::remove_file(temp_path);
/// ```
pub fn probe_avi_metadata<P: AsRef<Path>>(avi_path: P) -> Result<AviClipMetadata, CoreError> {
    let mut file = File::open(avi_path.as_ref())
        .map_err(|e| CoreError::Serialization(format!("Failed to open AVI file: {}", e)))?;
    let file_len = file
        .metadata()
        .map_err(|e| CoreError::Serialization(e.to_string()))?
        .len();

    if file_len < 88 {
        return Err(CoreError::Serialization(
            "AVI file is too small to contain valid RIFF headers".to_string(),
        ));
    }

    let hdr_read_len = (file_len as usize).min(4096);
    let mut hdr = vec![0u8; hdr_read_len];
    file.read_exact(&mut hdr)
        .map_err(|e| CoreError::Serialization(e.to_string()))?;

    if &hdr[0..4] != b"RIFF" || &hdr[8..12] != b"AVI " {
        return Err(CoreError::Serialization(
            "File is not a valid RIFF AVI container".to_string(),
        ));
    }

    let us_per_frame = u32::from_le_bytes([hdr[32], hdr[33], hdr[34], hdr[35]]);
    let avih_total_frames = u32::from_le_bytes([hdr[48], hdr[49], hdr[50], hdr[51]]);
    let width = u32::from_le_bytes([hdr[64], hdr[65], hdr[66], hdr[67]]);
    let height = u32::from_le_bytes([hdr[68], hdr[69], hdr[70], hdr[71]]);

    // Extract exact integer FPS from video 'strh' ('vidsMJPG') if present, falling back to us_per_frame
    let mut fps = if us_per_frame > 0 {
        (1_000_000u64 / u64::from(us_per_frame)).clamp(1, 240) as u32
    } else {
        30
    };

    if let Some(pos) = hdr.windows(8).position(|w| w == b"vidsMJPG") {
        if pos + 28 <= hdr.len() {
            let rate = u32::from_le_bytes([hdr[pos + 24], hdr[pos + 25], hdr[pos + 26], hdr[pos + 27]]);
            if (1..=240).contains(&rate) {
                fps = rate;
            }
        }
    }

    // Extract audio sample rate and channel count from audio 'strh'/'strf' if present
    let mut audio_sample_rate = 48_000u32;
    let mut audio_channels = 2u16;
    if let Some(pos) = hdr.windows(4).position(|w| w == b"auds") {
        if pos + 28 <= hdr.len() {
            let sr = u32::from_le_bytes([hdr[pos + 24], hdr[pos + 25], hdr[pos + 26], hdr[pos + 27]]);
            if sr >= 8_000 {
                audio_sample_rate = sr;
            }
        }
        if pos + 66 <= hdr.len() && &hdr[pos + 56..pos + 60] == b"strf" {
            let ch = u16::from_le_bytes([hdr[pos + 66], hdr[pos + 67]]);
            if (1..=8).contains(&ch) {
                audio_channels = ch;
            }
        }
    }

    let total_frames = avih_total_frames.max(1);
    let duration_secs = f64::from(total_frames) / f64::from(fps.max(1));

    Ok(AviClipMetadata {
        width: width.max(1),
        height: height.max(1),
        fps,
        total_frames,
        duration_secs,
        audio_sample_rate,
        audio_channels,
    })
}

/// Losslessly trims a recorded `.avi` video clip to `range` (`start_sec..end_sec`) without re-encoding
/// video or audio (`[REQ-TRIM-001.3]`, `[NFR-TRIM-001.1]`).
///
/// Copies `00dc` JPEG frames and synchronized `01wb` PCM audio slices directly into a newly indexed
/// OpenDML 2.0 + AVI 1.0 `.avi` container on disk.
///
/// # Arguments
///
/// * `src_avi_path` - Path to the source `.avi` file.
/// * `dst_avi_path` - Destination path for the trimmed `.avi` file.
/// * `range` - [`TrimRange`] specifying the In and Out timestamps in seconds.
///
/// # Returns
///
/// Returns the output [`PathBuf`] of the trimmed `.avi` file.
///
/// # Errors
///
/// Returns [`CoreError`] if the source file cannot be read, contains no video frames in the range,
/// or if writing the output container fails.
///
/// # Examples
///
/// ```rust
/// use rec_core::avi_trimmer::{trim_avi_lossless, TrimRange};
/// use rec_core::avi_writer::AviWriter;
/// use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
/// use rec_core::pixel_format::PixelFormat;
///
/// let temp_dir = std::env::temp_dir();
/// let src_path = temp_dir.join("doctest_trim_src.avi");
/// let dst_path = temp_dir.join("doctest_trim_dst.avi");
///
/// let mut writer = AviWriter::new(&src_path, 160, 120, 10).unwrap();
/// let desc = FrameDescriptor::new(160, 120, PixelFormat::Rgba).unwrap();
/// for i in 0..20u64 {
///     let frame = VideoFrame::new(desc.clone(), Pts::from_nanos(i * 100_000_000), vec![90u8; 160 * 120 * 4]).unwrap();
///     writer.write_frame(&frame).unwrap();
/// }
/// writer.finalize().unwrap();
///
/// let range = TrimRange::new(0.5, 1.5, 2.0);
/// let out = trim_avi_lossless(&src_path, &dst_path, range).unwrap();
/// assert!(out.exists());
///
/// let _ = std::fs::remove_file(src_path);
/// let _ = std::fs::remove_file(dst_path);
/// ```
pub fn trim_avi_lossless<P1: AsRef<Path>, P2: AsRef<Path>>(
    src_avi_path: P1,
    dst_avi_path: P2,
    range: TrimRange,
) -> Result<PathBuf, CoreError> {
    let meta = probe_avi_metadata(src_avi_path.as_ref())?;
    let sanitized = TrimRange::new(range.start_sec, range.end_sec, meta.duration_secs);
    let (start_frame, end_frame) = sanitized.frame_bounds(meta.fps, meta.total_frames);
    let (start_audio_byte, end_audio_byte) =
        sanitized.audio_byte_bounds(meta.audio_sample_rate, meta.audio_channels);
    let block_align = (u64::from(meta.audio_channels.max(1)) * 2).max(2);

    let mut src_file = File::open(src_avi_path.as_ref())
        .map_err(|e| CoreError::Serialization(format!("Failed to open source AVI: {}", e)))?;
    let file_len = src_file
        .metadata()
        .map_err(|e| CoreError::Serialization(e.to_string()))?
        .len();

    let mut dst_writer = AviWriter::new(dst_avi_path.as_ref(), meta.width, meta.height, meta.fps)?;
    dst_writer.set_audio_format(meta.audio_sample_rate, meta.audio_channels);

    let mut pos: u64 = 4096;
    if pos >= file_len {
        pos = 12;
    }

    let mut video_idx: u32 = 0;
    let mut audio_byte_cursor: u64 = 0;
    let mut frames_copied: u32 = 0;
    let mut chunk_hdr = [0u8; 8];
    let mut payload_buf = Vec::with_capacity(128 * 1024);

    while pos + 8 <= file_len {
        src_file
            .seek(SeekFrom::Start(pos))
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        if src_file.read_exact(&mut chunk_hdr).is_err() {
            break;
        }

        let tag = &chunk_hdr[0..4];
        let size = u32::from_le_bytes([
            chunk_hdr[4],
            chunk_hdr[5],
            chunk_hdr[6],
            chunk_hdr[7],
        ]) as u64;

        if tag == b"RIFF" || tag == b"LIST" {
            pos += 12;
            continue;
        }

        if tag == b"00dc" && size > 0 && pos + 8 + size <= file_len {
            if video_idx >= start_frame && video_idx < end_frame {
                payload_buf.resize(size as usize, 0);
                src_file
                    .read_exact(&mut payload_buf)
                    .map_err(|e| CoreError::Serialization(e.to_string()))?;
                dst_writer.write_preencoded_frame(&payload_buf)?;
                frames_copied += 1;
            }
            video_idx += 1;
        } else if tag == b"01wb" && size > 0 && pos + 8 + size <= file_len {
            let chunk_start = audio_byte_cursor;
            let chunk_end = audio_byte_cursor + size;
            if chunk_end > start_audio_byte && chunk_start < end_audio_byte {
                payload_buf.resize(size as usize, 0);
                src_file
                    .read_exact(&mut payload_buf)
                    .map_err(|e| CoreError::Serialization(e.to_string()))?;

                let rel_start = start_audio_byte.saturating_sub(chunk_start);
                let rel_end = (end_audio_byte - chunk_start).min(size);
                // Align slice boundaries to sample frame size (`block_align`)
                let aligned_start = ((rel_start / block_align) * block_align) as usize;
                let aligned_end = ((rel_end / block_align) * block_align) as usize;
                if aligned_end > aligned_start && aligned_end <= payload_buf.len() {
                    dst_writer.write_raw_pcm_chunk(&payload_buf[aligned_start..aligned_end])?;
                }
            }
            audio_byte_cursor = chunk_end;
        }

        if video_idx >= end_frame && audio_byte_cursor >= end_audio_byte {
            break;
        }

        let padded_size = size + (size & 1);
        if padded_size == 0 && tag == [0, 0, 0, 0] {
            break;
        }
        pos = pos.saturating_add(8).saturating_add(padded_size);
    }

    dst_writer.finalize()?;

    if frames_copied == 0 {
        return Err(CoreError::Serialization(
            "No video frames fell within the requested trim range".to_string(),
        ));
    }

    Ok(dst_avi_path.as_ref().to_path_buf())
}
