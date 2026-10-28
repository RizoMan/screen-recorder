//! `rec-engine`: Central multi-threaded pipeline orchestrator, DIP traits,
//! and circular instant replay buffer for Screen Recorder.
//!
//! # Architecture Role
//!
//! As defined in `[SPEC-ARCH-001]`, this crate operates as the central coordination layer
//! applying Dependency Inversion (DIP). It defines the interfaces (`CaptureBackend`, `AudioBackend`,
//! `EncoderBackend`, `MuxerBackend`) implemented by platform crates and delegates
//! high-speed frame processing through lock-free ring buffers.
//!
//! # Safety Invariant
//!
//! This crate strictly forbids unsafe code: `#![forbid(unsafe_code)]`.

#![forbid(unsafe_code)]

pub mod errors;
pub mod pipeline;
pub mod replay_buffer;
pub mod traits;

pub use errors::{AudioError, CaptureError, EncoderError, EngineError, MuxerError, ReplayError};
pub use pipeline::{RecordingPipeline, RecordingState};
pub use replay_buffer::InstantReplayBuffer;
pub use traits::{AudioBackend, CaptureBackend, CaptureTarget, EncoderBackend, MuxerBackend};
