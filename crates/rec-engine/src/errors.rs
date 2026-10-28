//! Typed error hierarchies for the capture engine, backends, and pipeline orchestration.
//!
//! Conforms to `[SPEC-ERR-001]` and `[CONTRACT-ERR-001]`.

/// Errors occurring during video capture operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum CaptureError {
    /// Target monitor, window, or area was not found or has been closed.
    #[error("Target monitor or window not found: {0}")]
    TargetNotFound(String),

    /// OS permission was denied for screen capture.
    #[error("Screen capture permission was denied by the operating system")]
    PermissionDenied,

    /// DirectX or GPU capture device was lost or removed.
    #[error("DirectX / GPU capture device lost or reset")]
    DeviceLost,

    /// Platform or API is not supported on this OS or hardware.
    #[error("Capture platform unsupported: {0}")]
    UnsupportedPlatform(String),

    /// Internal capture failure.
    #[error("Internal capture error: {0}")]
    Internal(String),
}

/// Errors occurring during audio recording operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum AudioError {
    /// Specified audio endpoint device was not found.
    #[error("Audio endpoint device not found: {0}")]
    DeviceNotFound(String),

    /// Audio stream initialization failed.
    #[error("Audio stream initialization failed: {0}")]
    StreamInitFailed(String),

    /// Unsupported audio channel or sample rate format.
    #[error("Unsupported audio format")]
    UnsupportedFormat,

    /// Internal audio capture error.
    #[error("Internal audio capture error: {0}")]
    Internal(String),
}

/// Errors occurring during hardware or software video/audio encoding.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum EncoderError {
    /// Encoder initialization failed.
    #[error("Encoder initialization failed: {0}")]
    InitializationFailed(String),

    /// Frame encoding failed.
    #[error("Frame encoding failed: {0}")]
    EncodingFailed(String),

    /// Hardware acceleration requested but unavailable.
    #[error("Hardware acceleration is unavailable on this system")]
    HardwareAccelerationUnavailable,

    /// Flushing pending encoder packets failed.
    #[error("Encoder flush failed: {0}")]
    FlushFailed(String),
}

/// Errors occurring during container multiplexing and disk writing.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum MuxerError {
    /// Failed to create destination media file.
    #[error("Failed to create container output file: {0}")]
    FileCreateFailed(String),

    /// Failed to write media packet to container.
    #[error("Failed to write packet to container: {0}")]
    WritePacketFailed(String),

    /// Finalizing the container index/trailer failed.
    #[error("Failed to finalize container: {0}")]
    FinalizeFailed(String),
}

/// Errors occurring during instant replay buffer operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// Replay buffer contains no recorded frames to export.
    #[error("Instant replay buffer is empty")]
    EmptyBuffer,

    /// Saving instant replay file failed.
    #[error("Failed to save instant replay: {0}")]
    SaveFailed(String),
}

/// High-level engine and pipeline orchestration errors.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Pipeline is already running.
    #[error("Recording pipeline is already active")]
    PipelineAlreadyRunning,

    /// Pipeline is not running.
    #[error("Recording pipeline is not active")]
    PipelineNotRunning,

    /// Wrapped capture error.
    #[error("Capture backend error: {0}")]
    Capture(#[from] CaptureError),

    /// Wrapped audio error.
    #[error("Audio backend error: {0}")]
    Audio(#[from] AudioError),

    /// Wrapped encoder error.
    #[error("Encoder error: {0}")]
    Encoder(#[from] EncoderError),

    /// Wrapped muxer error.
    #[error("Muxer error: {0}")]
    Muxer(#[from] MuxerError),

    /// Wrapped replay error.
    #[error("Instant replay error: {0}")]
    Replay(#[from] ReplayError),

    /// Core domain error.
    #[error("Core domain error: {0}")]
    Core(#[from] rec_core::errors::CoreError),
}
