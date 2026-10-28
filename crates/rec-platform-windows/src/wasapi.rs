//! WASAPI Audio loopback and microphone capture backend.
//!
//! Conforms to `[SPEC-PLAT-WIN-001]` and `[CONTRACT-ARCH-001]`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::Sender;
use rec_core::audio::AudioFrame;
use rec_core::frame::Pts;
use rec_engine::errors::AudioError;
use rec_engine::traits::AudioBackend;

/// WASAPI Audio capture backend implementing [`AudioBackend`].
pub struct WasapiAudioBackend {
    is_capturing: Arc<AtomicBool>,
    stream: Option<cpal::Stream>,
    epoch: Option<Instant>,
}

impl Default for WasapiAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WasapiAudioBackend {
    /// Creates a new inactive `WasapiAudioBackend`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
            stream: None,
            epoch: None,
        }
    }
}

impl AudioBackend for WasapiAudioBackend {
    fn set_epoch(&mut self, epoch: Instant) {
        self.epoch = Some(epoch);
    }

    fn start_audio_capture(&mut self, audio_tx: Sender<AudioFrame>) -> Result<(), AudioError> {
        if self.is_capturing.load(Ordering::SeqCst) {
            return Ok(());
        }

        let start_time = self.epoch.take().unwrap_or_else(Instant::now);

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .or_else(|| host.default_input_device())
            .ok_or_else(|| AudioError::DeviceNotFound("No default audio device found".to_string()))?;

        let supported_config = device
            .default_output_config()
            .map_err(|e| AudioError::StreamInitFailed(e.to_string()))?;

        let sample_rate = supported_config.sample_rate();
        let channels = supported_config.channels();

        let stream = device
            .build_input_stream(
                supported_config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let pts = Pts::from_duration(start_time.elapsed());
                    if let Ok(frame) = AudioFrame::new(sample_rate, channels, pts, data.to_vec()) {
                        let _ = audio_tx.send(frame);
                    }
                },
                move |err| {
                    log::error!("WASAPI stream error: {}", err);
                },
                None,
            )
            .map_err(|e| AudioError::StreamInitFailed(e.to_string()))?;

        stream.play().map_err(|e| AudioError::StreamInitFailed(e.to_string()))?;

        self.stream = Some(stream);
        self.is_capturing.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop_audio_capture(&mut self) -> Result<(), AudioError> {
        self.is_capturing.store(false, Ordering::SeqCst);
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        Ok(())
    }

    fn is_capturing(&self) -> bool {
        self.is_capturing.load(Ordering::SeqCst)
    }

    fn backend_name(&self) -> &'static str {
        "Windows WASAPI Audio Loopback"
    }
}

impl Drop for WasapiAudioBackend {
    fn drop(&mut self) {
        let _ = self.stop_audio_capture();
    }
}
