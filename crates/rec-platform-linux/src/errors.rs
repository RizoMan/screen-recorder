//! Linux platform-specific typed error definitions.
//!
//! Conforms to `[SPEC-ERR-001]` and `[SPEC-PLAT-LIN-001]`.

use rec_engine::errors::CaptureError;

/// Linux platform-specific capture errors.
#[derive(Debug, thiserror::Error)]
pub enum LinuxPlatformError {
    /// Failure communicating with `xdg-desktop-portal` DBus service.
    #[error("DBus portal screencast request failed: {0}")]
    PortalFailed(String),

    /// PipeWire stream creation or connection failure.
    #[error("PipeWire stream error: {0}")]
    PipeWireError(String),

    /// X11 / XCB capture failure.
    #[error("X11 capture error: {0}")]
    X11Error(String),

    /// Executed on non-Linux platform.
    #[error("Linux capture is unsupported on this platform")]
    UnsupportedPlatform,
}

impl From<LinuxPlatformError> for CaptureError {
    fn from(err: LinuxPlatformError) -> Self {
        match err {
            LinuxPlatformError::UnsupportedPlatform => {
                CaptureError::UnsupportedPlatform("Linux backends require a Linux kernel target".to_string())
            }
            LinuxPlatformError::PortalFailed(msg) => CaptureError::Internal(format!("Portal: {}", msg)),
            LinuxPlatformError::PipeWireError(msg) => CaptureError::Internal(format!("PipeWire: {}", msg)),
            LinuxPlatformError::X11Error(msg) => CaptureError::Internal(format!("X11: {}", msg)),
        }
    }
}
