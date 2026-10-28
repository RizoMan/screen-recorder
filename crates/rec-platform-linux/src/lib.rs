//! `rec-platform-linux`: Linux-native screen capture backends.
//!
//! # Architecture Role
//!
//! Conforms to `[SPEC-PLAT-LIN-001]`. Implements [`rec_engine::traits::CaptureBackend`]
//! for Linux using modern Wayland PipeWire portals and X11 legacy fallback.

pub mod errors;
pub mod pipewire;
pub mod x11;

pub use errors::LinuxPlatformError;
pub use pipewire::PipeWireCaptureBackend;
pub use x11::X11CaptureBackend;
