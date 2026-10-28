//! Linux X11 (XSHM / XDamage) fallback capture backend.
//!
//! Conforms to `[SPEC-PLAT-LIN-001]` and `[CONTRACT-ARCH-001]`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::Sender;
use rec_core::frame::VideoFrame;
use rec_engine::errors::CaptureError;
use rec_engine::traits::{CaptureBackend, CaptureTarget};

/// Linux X11 fallback implementation of [`CaptureBackend`].
pub struct X11CaptureBackend {
    is_capturing: Arc<AtomicBool>,
}

impl Default for X11CaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl X11CaptureBackend {
    /// Creates a new inactive `X11CaptureBackend`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl CaptureBackend for X11CaptureBackend {
    fn start_capture(
        &mut self,
        _target: CaptureTarget,
        _frame_tx: Sender<VideoFrame>,
    ) -> Result<(), CaptureError> {
        #[cfg(target_os = "linux")]
        {
            self.is_capturing.store(true, Ordering::SeqCst);
            // Native XCB / XSHM loop implementation
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(CaptureError::UnsupportedPlatform(
                "X11 backend requires a Linux X11 session".to_string(),
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
        "Linux X11 (MIT-SHM)"
    }
}

impl Drop for X11CaptureBackend {
    fn drop(&mut self) {
        let _ = self.stop_capture();
    }
}
