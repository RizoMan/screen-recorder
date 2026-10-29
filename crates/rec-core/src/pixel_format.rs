//! Pixel format definitions and buffer sizing algorithms.
//!
//! Conforms to `[SPEC-CORE-001]` and `[CONTRACT-CORE-001]`.

use crate::errors::CoreError;

/// Enumeration of supported raw video pixel formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PixelFormat {
    /// 32-bit Red, Green, Blue, Alpha (8 bits per channel).
    Rgba,
    /// 32-bit Blue, Green, Red, Alpha (standard for Direct3D / DXGI surfaces).
    Bgra,
    /// YUV 4:2:0 bi-planar format (Y plane followed by interleaved UV plane).
    Nv12,
    /// YUV 4:2:0 planar format (Y plane followed by U plane then V plane).
    I420,
}

impl PixelFormat {
    /// Returns the average bytes per pixel for packed formats, or 0 for planar formats.
    ///
    /// # Returns
    ///
    /// Returns 4 for RGBA and BGRA formats. For sub-sampled planar formats (NV12, I420),
    /// returns 1 (since the luma plane has 1 byte per pixel).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::pixel_format::PixelFormat;
    ///
    /// assert_eq!(PixelFormat::Rgba.bytes_per_pixel(), 4);
    /// assert_eq!(PixelFormat::Bgra.bytes_per_pixel(), 4);
    /// ```
    #[must_use]
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Rgba | Self::Bgra => 4,
            Self::Nv12 | Self::I420 => 1,
        }
    }

    /// Calculates the minimum required byte buffer size for a given resolution.
    ///
    /// # Arguments
    ///
    /// * `width` - Frame width in pixels.
    /// * `height` - Frame height in pixels.
    ///
    /// # Returns
    ///
    /// Returns the exact required byte capacity on success.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidDimensions`] if `width` or `height` is zero,
    /// or if integer multiplication overflows.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::pixel_format::PixelFormat;
    ///
    /// let size = PixelFormat::Rgba.calculate_buffer_size(1920, 1080).expect("valid size");
    /// assert_eq!(size, 1920 * 1080 * 4);
    /// ```
    pub fn calculate_buffer_size(&self, width: u32, height: u32) -> Result<usize, CoreError> {
        if width == 0 || height == 0 {
            return Err(CoreError::InvalidDimensions { width, height });
        }

        let w = width as usize;
        let h = height as usize;

        match self {
            Self::Rgba | Self::Bgra => {
                let pixels = w.checked_mul(h).ok_or(CoreError::InvalidDimensions { width, height })?;
                pixels.checked_mul(4).ok_or(CoreError::InvalidDimensions { width, height })
            }
            Self::Nv12 | Self::I420 => {
                // 1.5 bytes per pixel for YUV 4:2:0
                let pixels = w.checked_mul(h).ok_or(CoreError::InvalidDimensions { width, height })?;
                let chroma = pixels.checked_div(2).ok_or(CoreError::InvalidDimensions { width, height })?;
                pixels.checked_add(chroma).ok_or(CoreError::InvalidDimensions { width, height })
            }
        }
    }

    /// Calculates the minimum horizontal row stride in bytes.
    ///
    /// # Arguments
    ///
    /// * `width` - Frame width in pixels.
    ///
    /// # Returns
    ///
    /// Returns the row stride in bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidDimensions`] if `width` is zero or causes an overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::pixel_format::PixelFormat;
    ///
    /// let stride = PixelFormat::Rgba.row_stride(1920).expect("valid stride");
    /// assert_eq!(stride, 1920 * 4);
    /// ```
    pub fn row_stride(&self, width: u32) -> Result<usize, CoreError> {
        if width == 0 {
            return Err(CoreError::InvalidDimensions { width, height: 1 });
        }
        let w = width as usize;
        match self {
            Self::Rgba | Self::Bgra => w.checked_mul(4).ok_or(CoreError::InvalidDimensions { width, height: 1 }),
            Self::Nv12 | Self::I420 => Ok(w),
        }
    }
}

