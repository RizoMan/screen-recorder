//! `rec-platform-macos`: macOS-native screen capture backend using ScreenCaptureKit.
//!
//! # Architecture Role
//!
//! Conforms to `[SPEC-PLAT-MAC-001]`. Implements [`rec_engine::traits::CaptureBackend`]
//! for macOS 12.3+ utilizing ScreenCaptureKit with window exclusion lists.

pub mod errors;
pub mod screencapturekit;

pub use errors::MacPlatformError;
pub use screencapturekit::ScreenCaptureKitBackend;
