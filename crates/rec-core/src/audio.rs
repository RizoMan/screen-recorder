//! PCM Audio frames, mixing algorithms, and audio VU peak calculation.
//!
//! Conforms to `[SPEC-CORE-001]` and `[CONTRACT-CORE-001]`.

use crate::errors::CoreError;
use crate::frame::Pts;

/// Uncompressed 32-bit floating-point PCM audio frame.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioFrame {
    /// Audio sampling rate in Hertz (e.g., 48000).
    pub sample_rate: u32,
    /// Channel count (e.g., 1 for mono, 2 for stereo).
    pub channels: u16,
    /// Presentation timestamp of the first sample.
    pub pts: Pts,
    /// Interleaved PCM 32-bit float samples normalized between -1.0 and 1.0.
    pub samples: Vec<f32>,
}

impl AudioFrame {
    /// Constructs and validates a new [`AudioFrame`].
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Sample rate in Hz.
    /// * `channels` - Channel count.
    /// * `pts` - Presentation timestamp.
    /// * `samples` - Interleaved float samples.
    ///
    /// # Returns
    ///
    /// Validated audio frame.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidAudioConfig`] if `sample_rate` or `channels` is zero.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::audio::AudioFrame;
    /// use rec_core::frame::Pts;
    ///
    /// let frame = AudioFrame::new(48000, 2, Pts::from_nanos(0), vec![0.0; 960]).unwrap();
    /// assert_eq!(frame.channels, 2);
    /// assert_eq!(frame.sample_rate, 48000);
    /// ```
    pub fn new(sample_rate: u32, channels: u16, pts: Pts, samples: Vec<f32>) -> Result<Self, CoreError> {
        if sample_rate == 0 || channels == 0 {
            return Err(CoreError::InvalidAudioConfig { sample_rate, channels });
        }
        Ok(Self {
            sample_rate,
            channels,
            pts,
            samples,
        })
    }

    /// Computes the peak amplitude level per channel for audio VU meters.
    ///
    /// # Returns
    ///
    /// A tuple `(peak_left, peak_right)` clamped between 0.0 and 1.0.
    /// For mono audio, returns the same peak for both channels.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::audio::AudioFrame;
    /// use rec_core::frame::Pts;
    ///
    /// let frame = AudioFrame::new(48000, 2, Pts::from_nanos(0), vec![0.5, -0.8]).unwrap();
    /// let (left, right) = frame.calculate_peaks();
    /// assert!((left - 0.5).abs() < 1e-5);
    /// assert!((right - 0.8).abs() < 1e-5);
    /// ```
    #[must_use]
    pub fn calculate_peaks(&self) -> (f32, f32) {
        if self.samples.is_empty() || self.channels == 0 {
            return (0.0, 0.0);
        }

        let mut peak_l: f32 = 0.0;
        let mut peak_r: f32 = 0.0;

        if self.channels == 1 {
            for &sample in &self.samples {
                let abs = sample.abs();
                if abs > peak_l {
                    peak_l = abs;
                }
            }
            peak_r = peak_l;
        } else {
            for (idx, &sample) in self.samples.iter().enumerate() {
                let abs = sample.abs();
                if idx % 2 == 0 {
                    if abs > peak_l {
                        peak_l = abs;
                    }
                } else if abs > peak_r {
                    peak_r = abs;
                }
            }
        }

        (peak_l.clamp(0.0, 1.0), peak_r.clamp(0.0, 1.0))
    }

    /// Applies an in-place gain multiplier to this frame's samples.
    ///
    /// # Arguments
    ///
    /// * `gain` - Multiplier factor (e.g. `1.5` for +50%, `0.0` for silence).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::audio::AudioFrame;
    /// use rec_core::frame::Pts;
    ///
    /// let mut frame = AudioFrame::new(48000, 2, Pts::from_nanos(0), vec![0.4, -0.4]).unwrap();
    /// frame.apply_gain(0.5);
    /// assert!((frame.samples[0] - 0.2).abs() < 1e-5);
    /// ```
    pub fn apply_gain(&mut self, gain: f32) {
        AudioMixer::apply_gain(&mut self.samples, gain);
    }
}

/// Software audio mixer for combining multiple PCM float streams.
#[derive(Debug, Default)]
pub struct AudioMixer;

impl AudioMixer {
    /// Mixes two stereo PCM float buffers into the primary buffer, applying volume gains.
    ///
    /// # Arguments
    ///
    /// * `primary` - In-place destination buffer.
    /// * `primary_gain` - Volume multiplier for primary stream.
    /// * `secondary` - Secondary buffer to mix in.
    /// * `secondary_gain` - Volume multiplier for secondary stream.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::audio::AudioMixer;
    ///
    /// let mut track1 = vec![0.2, 0.2];
    /// let track2 = vec![0.4, 0.4];
    /// AudioMixer::mix_buffers(&mut track1, 1.0, &track2, 1.0);
    /// assert!((track1[0] - 0.6).abs() < 1e-5);
    /// ```
    pub fn mix_buffers(primary: &mut [f32], primary_gain: f32, secondary: &[f32], secondary_gain: f32) {
        let len = primary.len().min(secondary.len());
        for i in 0..len {
            let mixed = (primary[i] * primary_gain) + (secondary[i] * secondary_gain);
            // Soft clipping / clamping to avoid harsh digital distortion
            primary[i] = mixed.clamp(-1.0, 1.0);
        }
    }

    /// Applies a volume gain multiplier in-place to an audio buffer, clamping samples between -1.0 and 1.0.
    ///
    /// # Arguments
    ///
    /// * `buffer` - In-place audio sample buffer.
    /// * `gain` - Multiplier factor.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::audio::AudioMixer;
    ///
    /// let mut samples = vec![0.5, -0.5];
    /// AudioMixer::apply_gain(&mut samples, 1.5);
    /// assert!((samples[0] - 0.75).abs() < 1e-5);
    /// assert!((samples[1] + 0.75).abs() < 1e-5);
    /// ```
    pub fn apply_gain(buffer: &mut [f32], gain: f32) {
        if (gain - 1.0).abs() < 1e-4 {
            return;
        }
        for sample in buffer.iter_mut() {
            *sample = (*sample * gain).clamp(-1.0, 1.0);
        }
    }
}
