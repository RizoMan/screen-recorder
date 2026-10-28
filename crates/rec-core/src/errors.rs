//! Error types for the core screen recorder domain.
//!
//! Conforms to `[SPEC-ERR-001]` and `[CONTRACT-ERR-001]`.

/// Unified error enumeration for domain models and operations in `rec-core`.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CoreError {
    /// Raised when invalid frame dimensions are provided (e.g., zero width or height).
    #[error("Invalid frame dimensions: width={width}, height={height}")]
    InvalidDimensions {
        /// The invalid width in pixels.
        width: u32,
        /// The invalid height in pixels.
        height: u32,
    },

    /// Raised when the frame data buffer size does not match the descriptor requirements.
    #[error("Invalid buffer size: expected {expected} bytes, got {actual} bytes")]
    InvalidBufferSize {
        /// Expected byte length.
        expected: usize,
        /// Actual provided byte length.
        actual: usize,
    },

    /// Raised when a ring buffer reaches its configured capacity.
    #[error("Ring buffer capacity exceeded: maximum capacity is {capacity}")]
    BufferOverflow {
        /// Maximum capacity of the ring buffer.
        capacity: usize,
    },

    /// Raised when popping from an empty ring buffer.
    #[error("Ring buffer is empty")]
    BufferEmpty,

    /// Raised when an invalid audio sample rate or channel configuration is detected.
    #[error("Invalid audio configuration: sample_rate={sample_rate}, channels={channels}")]
    InvalidAudioConfig {
        /// Sampling rate in Hz.
        sample_rate: u32,
        /// Number of audio channels.
        channels: u16,
    },

    /// Raised when audio mixing formats do not match.
    #[error("Audio format mismatch: expected {expected_sample_rate}Hz, got {actual_sample_rate}Hz")]
    AudioFormatMismatch {
        /// Expected sample rate in Hz.
        expected_sample_rate: u32,
        /// Actual sample rate in Hz.
        actual_sample_rate: u32,
    },

    /// Raised when configuration validation fails.
    #[error("Configuration validation error: {0}")]
    ConfigValidation(String),

    /// Raised during configuration serialization or deserialization.
    #[error("Serialization error: {0}")]
    Serialization(String),
}
