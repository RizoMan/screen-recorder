//! macOS platform-specific typed error definitions.
//!
//! Conforms to `[SPEC-ERR-001]` and `[SPEC-PLAT-MAC-001]`.

use rec_engine::errors::CaptureError;

/// macOS platform-specific capture errors.
#[derive(Debug, thiserror::Error)]
pub enum MacPlatformError {
    /// Failure initializing ScreenCaptureKit session.
    #[error("ScreenCaptureKit initialization failed: {0}")]
    ScreenCaptureKitError(String),

    /// Missing screen recording entitlements / permission.
    #[error("macOS Screen Recording permission denied")]
    PermissionDenied,

    /// Executed on non-macOS platform.
    #[error("macOS capture is unsupported on this platform")]
    UnsupportedPlatform,
}

impl From<MacPlatformError> for CaptureError {
    fn from(err: MacPlatformError) -> Self {
        match err {
            MacPlatformError::PermissionDenied => CaptureError::PermissionDenied,
            MacPlatformError::UnsupportedPlatform => {
                CaptureError::UnsupportedPlatform("ScreenCaptureKit requires macOS 12.3+".to_string())
            }
            MacPlatformError::ScreenCaptureKitError(msg) => {
                CaptureError::Internal(format!("ScreenCaptureKit: {}", msg))
            }
        }
    }
}
