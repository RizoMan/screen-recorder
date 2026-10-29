//! `rec-core`: Pure domain models, video frame definitions, audio mixing, ring buffers,
//! configuration management, and zero-allocation bilingual internationalization for Screen Recorder.
//!
//! # Architecture Role
//!
//! As defined in `[SPEC-ARCH-001]` and `[SPEC-CORE-001]`, this crate constitutes the innermost
//! domain layer of Clean Architecture (Domain Entity Layer). It has zero dependencies on
//! operating system kernel APIs or hardware-specific drivers.
//!
//! # Safety Invariant
//!
//! This crate strictly forbids unsafe code: `#![forbid(unsafe_code)]`.

#![forbid(unsafe_code)]

pub mod audio;
pub mod avi_trimmer;
pub mod avi_writer;
pub mod config;
pub mod cursor_effects;
pub mod errors;
pub mod frame;
pub mod gif_exporter;
pub mod i18n;
pub mod pixel_format;
pub mod ring_buffer;

pub use audio::{AudioFrame, AudioMixer};
pub use avi_trimmer::{probe_avi_metadata, trim_avi_lossless, AviClipMetadata, TrimRange};
pub use avi_writer::{encode_frame_to_jpeg, AviWriter};
pub use config::AppConfig;
pub use cursor_effects::{
    apply_cursor_effects, ClickButton, ClickRipple, CursorHighlightColor, CursorOverlayState,
};
pub use errors::CoreError;
pub use frame::{Dts, FrameDescriptor, Pts, VideoFrame};
pub use gif_exporter::{export_avi_to_gif, GifExportOptions, GifScale, GifWriter};
pub use i18n::{text, I18nKey, Language};
pub use pixel_format::PixelFormat;
pub use ring_buffer::LockFreeRingBuffer;
