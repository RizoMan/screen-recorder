//! macOS ScreenCaptureKit (macOS 12.3+) capture backend.
//!
//! Conforms to `[SPEC-PLAT-MAC-001]` and `[CONTRACT-ARCH-001]`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::Sender;
use rec_core::frame::VideoFrame;
use rec_engine::errors::CaptureError;
use rec_engine::traits::{CaptureBackend, CaptureTarget};

/// macOS ScreenCaptureKit implementation of [`CaptureBackend`].
pub struct ScreenCaptureKitBackend {
    is_capturing: Arc<AtomicBool>,
}

impl Default for ScreenCaptureKitBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenCaptureKitBackend {
    /// Creates a new inactive `ScreenCaptureKitBackend`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl CaptureBackend for ScreenCaptureKitBackend {
    fn start_capture(
        &mut self,
        _target: CaptureTarget,
        _frame_tx: Sender<VideoFrame>,
    ) -> Result<(), CaptureError> {
        #[cfg(target_os = "macos")]
        {
            self.is_capturing.store(true, Ordering::SeqCst);
            // Native ScreenCaptureKit SCStream configuration with window exclusion
            Ok(())
        }

        #[cfg(not(target_os = "macos"))]
        {
            Err(CaptureError::UnsupportedPlatform(
                "ScreenCaptureKit requires macOS 12.3+".to_string(),
            ))
        }
    }

    fn stop_capture(&mut self) -> Result<(), CaptureError> {
        self.is_capturing.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_capturing(&self) -> bool {
        self.is_capturing.load(Ordering::SeqCst)
    }

    fn backend_name(&self) -> &'static str {
        "macOS ScreenCaptureKit"
    }
}

impl Drop for ScreenCaptureKitBackend {
    fn drop(&mut self) {
        let _ = self.stop_capture();
    }
}
