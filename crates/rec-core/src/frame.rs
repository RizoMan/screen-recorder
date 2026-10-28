//! Video frame representations, descriptors, and presentation timestamps.
//!
//! Conforms to `[SPEC-CORE-001]` and `[CONTRACT-CORE-001]`.

use std::time::Duration;
use crate::errors::CoreError;
use crate::pixel_format::PixelFormat;

/// Presentation Timestamp (PTS) measured in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, serde::Serialize, serde::Deserialize)]
pub struct Pts(pub u64);

impl Pts {
    /// Creates a new [`Pts`] from raw nanoseconds.
    ///
    /// # Arguments
    ///
    /// * `nanos` - Monotonic timestamp in nanoseconds.
    ///
    /// # Returns
    ///
    /// A new [`Pts`] instance wrapping `nanos`.
    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Creates a new [`Pts`] from a standard [`Duration`].
    ///
    /// # Arguments
    ///
    /// * `duration` - Elapsed time duration.
    ///
    /// # Returns
    ///
    /// A [`Pts`] representing the duration in nanoseconds.
    #[must_use]
    pub const fn from_duration(duration: Duration) -> Self {
        Self(duration.as_nanos() as u64)
    }

    /// Converts the [`Pts`] into a standard [`Duration`].
    ///
    /// # Returns
    ///
    /// A [`Duration`] corresponding to this timestamp's nanoseconds.
    #[must_use]
    pub const fn as_duration(&self) -> Duration {
        Duration::from_nanos(self.0)
    }

    /// Returns the raw timestamp value in nanoseconds.
    ///
    /// # Returns
    ///
    /// Nanosecond count as `u64`.
    #[must_use]
    pub const fn as_nanos(&self) -> u64 {
        self.0
    }
}

/// Decode Timestamp (DTS) measured in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, serde::Serialize, serde::Deserialize)]
pub struct Dts(pub u64);

/// Metadata descriptor describing geometry and layout of a video frame.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameDescriptor {
    /// Horizontal width in pixels.
    pub width: u32,
    /// Vertical height in pixels.
    pub height: u32,
    /// Memory stride (bytes per scanline).
    pub stride: usize,
    /// Pixel format encoding.
    pub format: PixelFormat,
}

impl FrameDescriptor {
    /// Constructs and validates a new `FrameDescriptor`.
    ///
    /// # Arguments
    ///
    /// * `width` - Width in pixels.
    /// * `height` - Height in pixels.
    /// * `format` - Target [`PixelFormat`].
    ///
    /// # Returns
    ///
    /// Returns a validated descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidDimensions`] if `width` or `height` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::frame::FrameDescriptor;
    /// use rec_core::pixel_format::PixelFormat;
    ///
    /// let desc = FrameDescriptor::new(1920, 1080, PixelFormat::Rgba).expect("valid descriptor");
    /// assert_eq!(desc.width, 1920);
    /// assert_eq!(desc.stride, 1920 * 4);
    /// ```
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Result<Self, CoreError> {
        let stride = format.row_stride(width)?;
        let _ = format.calculate_buffer_size(width, height)?;
        Ok(Self {
            width,
            height,
            stride,
            format,
        })
    }

    /// Returns the minimum expected buffer size in bytes for this frame descriptor.
    ///
    /// # Returns
    ///
    /// Total buffer size in bytes.
    #[must_use]
    pub fn required_buffer_size(&self) -> usize {
        self.format
            .calculate_buffer_size(self.width, self.height)
            .unwrap_or(0)
    }
}

/// An uncompressed video frame with presentation timestamp and owned or shared memory buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFrame {
    /// Frame geometric and format descriptor.
    pub descriptor: FrameDescriptor,
    /// Presentation timestamp for synchronization.
    pub pts: Pts,
    /// Contiguous byte buffer containing raw pixel data.
    pub data: Vec<u8>,
}

impl VideoFrame {
    /// Constructs a new validated `VideoFrame`.
    ///
    /// # Arguments
    ///
    /// * `descriptor` - Geometric and pixel format descriptor.
    /// * `pts` - Monotonic presentation timestamp.
    /// * `data` - Raw pixel byte buffer.
    ///
    /// # Returns
    ///
    /// Returns the constructed frame.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidBufferSize`] if `data.len()` does not equal the required size.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::frame::{FrameDescriptor, VideoFrame, Pts};
    /// use rec_core::pixel_format::PixelFormat;
    ///
    /// let desc = FrameDescriptor::new(2, 2, PixelFormat::Rgba).unwrap();
    /// let buffer = vec![255u8; desc.required_buffer_size()];
    /// let frame = VideoFrame::new(desc, Pts::from_nanos(1_000_000), buffer).unwrap();
    /// assert_eq!(frame.data.len(), 16);
    /// ```
    pub fn new(descriptor: FrameDescriptor, pts: Pts, data: Vec<u8>) -> Result<Self, CoreError> {
        let expected = descriptor.required_buffer_size();
        if data.len() < expected {
            return Err(CoreError::InvalidBufferSize {
                expected,
                actual: data.len(),
            });
        }
        Ok(Self {
            descriptor,
            pts,
            data,
        })
    }

    /// Encodes and saves this video frame as a PNG image to disk.
    ///
    /// # Arguments
    ///
    /// * `path` - Destination image file path.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Serialization`] if encoding or file writing fails.
    pub fn save_as_png<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), CoreError> {
        let width = self.descriptor.width;
        let height = self.descriptor.height;

        let rgba_data = match self.descriptor.format {
            PixelFormat::Bgra => {
                let mut rgba = Vec::with_capacity(self.data.len());
                for chunk in self.data.chunks_exact(4) {
                    rgba.push(chunk[2]);
                    rgba.push(chunk[1]);
                    rgba.push(chunk[0]);
                    rgba.push(chunk[3]);
                }
                rgba
            }
            _ => self.data.clone(),
        };

        if let Some(parent) = path.as_ref().parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        image::save_buffer(
            path.as_ref(),
            &rgba_data,
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| CoreError::Serialization(format!("Failed to save screenshot PNG: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_video_frame_construction() {
        let desc = FrameDescriptor::new(100, 100, PixelFormat::Rgba).unwrap();
        let expected_size = desc.required_buffer_size();
        let data = vec![0u8; expected_size];
        let frame = VideoFrame::new(desc, Pts::from_nanos(100), data);
        assert!(frame.is_ok());
    }

    #[test]
    fn test_short_buffer_returns_error() {
        let desc = FrameDescriptor::new(100, 100, PixelFormat::Rgba).unwrap();
        let short_data = vec![0u8; 10];
        let frame = VideoFrame::new(desc, Pts::from_nanos(100), short_data);
        assert!(matches!(frame, Err(CoreError::InvalidBufferSize { .. })));
    }
}
