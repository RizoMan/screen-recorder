//! Unified abstractions and traits for screen capture, audio, and encoding.
//!
//! Conforms to `[SPEC-ARCH-001]`, `[CONTRACT-ARCH-001]`, and Dependency Inversion Principle (DIP).

use crossbeam_channel::Sender;
use rec_core::audio::AudioFrame;
use rec_core::config::AppConfig;
use rec_core::frame::{Pts, VideoFrame};
use crate::errors::{AudioError, CaptureError, EncoderError, MuxerError};

/// Target surface specification for screen capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureTarget {
    /// Captures the entirety of a specific monitor display.
    FullScreen {
        /// Monitor index (0-indexed).
        monitor_index: usize,
    },
    /// Captures a specific top-level window.
    Window {
        /// Platform-specific native window handle / ID.
        window_id: u64,
    },
    /// Captures a rectangular sub-region on the desktop.
    Area {
        /// Top-left X coordinate.
        x: u32,
        /// Top-left Y coordinate.
        y: u32,
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
}

/// Unified trait for video capture backends across all supported platforms.
///
/// Follows Dependency Inversion: concrete implementations in `rec-platform-*` implement this trait.
pub trait CaptureBackend: Send + Sync {
    /// Begins capturing frames from the target and sending them into `frame_tx`.
    ///
    /// # Arguments
    ///
    /// * `target` - Display, window, or rectangular sub-region to capture.
    /// * `frame_tx` - Channel sender for streaming uncompressed [`VideoFrame`]s.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError`] if the target cannot be accessed or device initialization fails.
    fn start_capture(
        &mut self,
        target: CaptureTarget,
        frame_tx: Sender<VideoFrame>,
    ) -> Result<(), CaptureError>;

    /// Stops the active capture session and halts frame dispatch.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError`] if session termination fails.
    fn stop_capture(&mut self) -> Result<(), CaptureError>;

    /// Checks if the backend is actively capturing frames.
    fn is_capturing(&self) -> bool;

    /// Configures the desired capture framerate.
    ///
    /// Default implementation does nothing.
    fn set_fps(&mut self, _fps: u32) {}

    /// Sets a shared session epoch timestamp for unified monotonic A/V synchronization.
    ///
    /// Default implementation does nothing.
    fn set_epoch(&mut self, _epoch: std::time::Instant) {}

    /// Updates live capture configuration (such as cursor halo, click ripples, and FPS).
    ///
    /// Default implementation does nothing.
    fn update_config(&mut self, _config: &AppConfig) {}

    /// Returns human-readable name of the backend (e.g., "Windows Graphics Capture", "PipeWire").
    fn backend_name(&self) -> &'static str;
}

/// Unified trait for audio recording backends (loopback system sound and microphone).
pub trait AudioBackend: Send + Sync {
    /// Begins capturing audio and sending PCM frames into `audio_tx`.
    ///
    /// # Arguments
    ///
    /// * `audio_tx` - Channel sender for streaming [`AudioFrame`]s.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError`] if audio devices cannot be opened.
    fn start_audio_capture(&mut self, audio_tx: Sender<AudioFrame>) -> Result<(), AudioError>;

    /// Stops the active audio capture session.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError`] if audio teardown fails.
    fn stop_audio_capture(&mut self) -> Result<(), AudioError>;

    /// Checks if audio capture is actively running.
    fn is_capturing(&self) -> bool;

    /// Sets a shared session epoch timestamp for unified monotonic A/V synchronization.
    ///
    /// Default implementation does nothing.
    fn set_epoch(&mut self, _epoch: std::time::Instant) {}

    /// Returns human-readable name of the audio backend (e.g., "WASAPI Loopback", "CoreAudio").
    fn backend_name(&self) -> &'static str;
}

/// Unified trait for video and audio encoders (hardware-accelerated or software fallback).
pub trait EncoderBackend: Send + Sync {
    /// Initializes encoder with the supplied configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Application recording configuration.
    ///
    /// # Errors
    ///
    /// Returns [`EncoderError`] if codec initialization fails.
    fn init(&mut self, config: &AppConfig) -> Result<(), EncoderError>;

    /// Encodes a raw video frame into compressed NAL units / packets.
    ///
    /// # Arguments
    ///
    /// * `frame` - Uncompressed input frame.
    ///
    /// # Errors
    ///
    /// Returns [`EncoderError`] if hardware or software encoding fails.
    fn encode_frame(&mut self, frame: VideoFrame) -> Result<(), EncoderError>;

    /// Encodes an audio frame.
    ///
    /// # Arguments
    ///
    /// * `frame` - Uncompressed input audio frame.
    ///
    /// # Errors
    ///
    /// Returns [`EncoderError`] on compression failure.
    fn encode_audio(&mut self, frame: AudioFrame) -> Result<(), EncoderError>;

    /// Flushes any buffered packets remaining in the encoder pipeline.
    ///
    /// # Errors
    ///
    /// Returns [`EncoderError`] on flush failure.
    fn flush(&mut self) -> Result<(), EncoderError>;
}

/// Unified trait for container multiplexers (MP4, MKV).
pub trait MuxerBackend: Send + Sync {
    /// Writes an encoded video packet to the destination container.
    ///
    /// # Arguments
    ///
    /// * `pts` - Presentation timestamp.
    /// * `is_keyframe` - True if packet is an IDR / I-frame.
    /// * `data` - Compressed bitstream packet data.
    ///
    /// # Errors
    ///
    /// Returns [`MuxerError`] if packet writing fails.
    fn write_video_packet(&mut self, pts: Pts, is_keyframe: bool, data: &[u8]) -> Result<(), MuxerError>;

    /// Writes an encoded audio packet to the destination container.
    ///
    /// # Arguments
    ///
    /// * `pts` - Presentation timestamp.
    /// * `data` - Compressed audio packet data.
    ///
    /// # Errors
    ///
    /// Returns [`MuxerError`] if audio writing fails.
    fn write_audio_packet(&mut self, pts: Pts, data: &[u8]) -> Result<(), MuxerError>;

    /// Finalizes container headers and writes index table.
    ///
    /// # Errors
    ///
    /// Returns [`MuxerError`] on finalization failure.
    fn finalize(&mut self) -> Result<(), MuxerError>;
}
