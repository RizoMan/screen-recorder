//! Windows platform-specific typed error conversions.
//!
//! Conforms to `[SPEC-ERR-001]` and `[SPEC-PLAT-WIN-001]`.

use rec_engine::errors::{AudioError, CaptureError};

/// Specific error types encountered when interacting with Windows APIs.
#[derive(Debug, thiserror::Error)]
pub enum WindowsPlatformError {
    /// Failure enumerating monitors or windows.
    #[error("Failed to enumerate display targets: {0}")]
    TargetEnumerationFailed(String),

    /// Direct3D 11 or DXGI capture error.
    #[error("Direct3D / DXGI error: {0}")]
    DxgiError(String),

    /// Windows Graphics Capture (WGC) session error.
    #[error("Windows Graphics Capture error: {0}")]
    WgcError(String),

    /// WASAPI audio client error.
    #[error("WASAPI audio error: {0}")]
    WasapiError(String),
}

impl From<WindowsPlatformError> for CaptureError {
    fn from(err: WindowsPlatformError) -> Self {
        match err {
            WindowsPlatformError::TargetEnumerationFailed(msg) => CaptureError::TargetNotFound(msg),
            WindowsPlatformError::DxgiError(msg) => CaptureError::Internal(format!("DXGI: {}", msg)),
            WindowsPlatformError::WgcError(msg) => CaptureError::Internal(format!("WGC: {}", msg)),
            WindowsPlatformError::WasapiError(msg) => CaptureError::Internal(format!("WASAPI: {}", msg)),
        }
    }
}

impl From<WindowsPlatformError> for AudioError {
    fn from(err: WindowsPlatformError) -> Self {
        match err {
            WindowsPlatformError::WasapiError(msg) => AudioError::StreamInitFailed(msg),
            _ => AudioError::Internal(err.to_string()),
        }
    }
}
