//! Persistent, sanitized application configuration.
//!
//! Conforms to `[SPEC-CORE-001]` and `[CONTRACT-CORE-001]`.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::cursor_effects::CursorHighlightColor;
use crate::errors::CoreError;
use crate::gif_exporter::GifScale;
use crate::i18n::Language;

/// Application configuration with sanitation and persistence support.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    /// Target capture framerate in frames per second (e.g., 30, 60, 120).
    pub fps: u32,
    /// Video encoding quality factor (CRF: 0 = lossless, 51 = worst, default 23).
    pub quality_crf: u8,
    /// Flag indicating whether system loopback audio is captured.
    pub record_system_audio: bool,
    /// Flag indicating whether microphone input is recorded.
    pub record_microphone: bool,
    /// System audio volume multiplier percentage (0 to 150, default 100%).
    pub system_audio_volume_percent: u32,
    /// Microphone input gain multiplier percentage (0 to 200, default 100%).
    pub microphone_gain_percent: u32,
    /// Flag indicating whether microphone input is temporarily muted.
    pub microphone_muted: bool,
    /// Current UI/CLI display language.
    pub language: Language,
    /// Default output directory path for recorded video files.
    pub output_directory: PathBuf,
    /// Duration in seconds of the circular instant replay memory buffer.
    pub instant_replay_duration_secs: u32,
    /// Enables GPU hardware acceleration (NVENC, QuickSync, AMF, VideoToolbox, VA-API).
    pub use_hardware_acceleration: bool,
    /// UI font and element display scale percentage (75% to 200%, default 100%).
    pub font_scale_percent: u32,
    /// Enables translucent visual highlight halo around the mouse cursor (`[REQ-CURSOR-001.1]`).
    #[serde(default)]
    pub cursor_halo_enabled: bool,
    /// Enables expanding concentric ripple waves on mouse left/right clicks (`[REQ-CURSOR-001.2]`).
    #[serde(default)]
    pub cursor_click_ripples_enabled: bool,
    /// Radius of the cursor highlight halo in pixels (12 to 64, default 28).
    #[serde(default = "default_cursor_halo_radius")]
    pub cursor_halo_radius: u32,
    /// Selected color preset for the cursor highlight halo.
    #[serde(default)]
    pub cursor_halo_color: CursorHighlightColor,
    /// Target frame rate for animated GIF exports (`10`, `15`, or `30` FPS, `[REQ-GIF-001.3]`).
    #[serde(default = "default_gif_fps")]
    pub gif_fps: u32,
    /// Resolution scaling preset for animated GIF exports (`[REQ-GIF-001.3]`).
    #[serde(default)]
    pub gif_scale: GifScale,
    /// Maximum clip duration in seconds for animated GIF exports (`5..=15`, default `10`).
    #[serde(default = "default_gif_max_duration_secs")]
    pub gif_max_duration_secs: u32,
}

const fn default_cursor_halo_radius() -> u32 {
    28
}

const fn default_gif_fps() -> u32 {
    15
}

const fn default_gif_max_duration_secs() -> u32 {
    10
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_output = if let Ok(profile) = std::env::var("USERPROFILE") {
            PathBuf::from(profile).join("Videos").join("ScreenRecorder")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join("Videos").join("ScreenRecorder")
        } else {
            PathBuf::from("recordings")
        };

        Self {
            fps: 30,
            quality_crf: 23,
            record_system_audio: true,
            record_microphone: false,
            system_audio_volume_percent: 100,
            microphone_gain_percent: 100,
            microphone_muted: false,
            language: Language::English,
            output_directory: default_output,
            instant_replay_duration_secs: 30,
            use_hardware_acceleration: true,
            font_scale_percent: 100,
            cursor_halo_enabled: false,
            cursor_click_ripples_enabled: true,
            cursor_halo_radius: 28,
            cursor_halo_color: CursorHighlightColor::YellowGlow,
            gif_fps: 15,
            gif_scale: GifScale::Scale75,
            gif_max_duration_secs: 10,
        }
    }
}

impl AppConfig {
    /// Creates and sanitizes a default [`AppConfig`] instance.
    ///
    /// # Returns
    ///
    /// Returns a newly initialized [`AppConfig`] with all fields clamped to safe operating ranges.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::config::AppConfig;
    ///
    /// let config = AppConfig::new();
    /// assert_eq!(config.fps, 30);
    /// assert!(config.validate().is_ok());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let mut config = Self::default();
        config.sanitize();
        config
    }

    /// Sanitizes configuration values in-place to ensure they fall within safe operating boundaries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::config::AppConfig;
    ///
    /// let mut config = AppConfig { fps: 999, quality_crf: 80, ..Default::default() };
    /// config.sanitize();
    /// assert_eq!(config.fps, 240);
    /// assert_eq!(config.quality_crf, 51);
    /// ```
    pub fn sanitize(&mut self) {
        // Clamp framerate between 15 and 240 FPS
        self.fps = self.fps.clamp(15, 240);
        // Clamp CRF quality between 0 and 51
        self.quality_crf = self.quality_crf.clamp(0, 51);
        // Clamp instant replay duration between 5 and 300 seconds
        self.instant_replay_duration_secs = self.instant_replay_duration_secs.clamp(5, 300);
        // Clamp UI font scale percentage between 75% and 200%
        self.font_scale_percent = self.font_scale_percent.clamp(75, 200);
        // Clamp system audio volume between 0% and 150%
        self.system_audio_volume_percent = self.system_audio_volume_percent.clamp(0, 150);
        // Clamp microphone gain between 0% and 200%
        self.microphone_gain_percent = self.microphone_gain_percent.clamp(0, 200);
        // Clamp cursor halo radius between 12 and 64 pixels
        self.cursor_halo_radius = self.cursor_halo_radius.clamp(12, 64);
        // Clamp GIF export FPS between 5 and 30 FPS
        self.gif_fps = self.gif_fps.clamp(5, 30);
        // Clamp GIF max duration between 5 and 15 seconds
        self.gif_max_duration_secs = self.gif_max_duration_secs.clamp(5, 15);
    }

    /// Validates the configuration and returns an error if any domain invariant is violated.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when all parameters satisfy domain invariants.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ConfigValidation`] if `fps` is outside `15..=240` or `quality_crf > 51`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::config::AppConfig;
    ///
    /// let config = AppConfig::new();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.fps < 15 || self.fps > 240 {
            return Err(CoreError::ConfigValidation(format!(
                "FPS must be between 15 and 240, got {}",
                self.fps
            )));
        }
        if self.quality_crf > 51 {
            return Err(CoreError::ConfigValidation(format!(
                "Quality CRF must be <= 51, got {}",
                self.quality_crf
            )));
        }
        Ok(())
    }

    /// Serializes configuration to a pretty-printed JSON string.
    ///
    /// # Returns
    ///
    /// Returns a UTF-8 JSON string representation of [`AppConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Serialization`] on JSON encoding failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::config::AppConfig;
    ///
    /// let config = AppConfig::new();
    /// let json = config.to_json().unwrap();
    /// assert!(json.contains("\"fps\": 30"));
    /// ```
    pub fn to_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// Deserializes, sanitizes, and validates an [`AppConfig`] from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `json_str` - The UTF-8 JSON payload to parse.
    ///
    /// # Returns
    ///
    /// Returns a sanitized and validated [`AppConfig`] instance.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Serialization`] if the JSON is malformed or [`CoreError::ConfigValidation`]
    /// if post-sanitization validation fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rec_core::config::AppConfig;
    ///
    /// let original = AppConfig::new();
    /// let json = original.to_json().unwrap();
    /// let restored = AppConfig::from_json(&json).unwrap();
    /// assert_eq!(original, restored);
    /// ```
    pub fn from_json(json_str: &str) -> Result<Self, CoreError> {
        let mut config: Self =
            serde_json::from_str(json_str).map_err(|e| CoreError::Serialization(e.to_string()))?;
        config.sanitize();
        config.validate()?;
        Ok(config)
    }

    /// Updates the destination directory path for recorded media files.
    ///
    /// # Arguments
    ///
    /// * `path` - New target directory path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use rec_core::config::AppConfig;
    ///
    /// let mut config = AppConfig::new();
    /// config.set_output_directory("C:/Recordings");
    /// assert_eq!(config.output_directory, Path::new("C:/Recordings"));
    /// ```
    pub fn set_output_directory<P: AsRef<Path>>(&mut self, path: P) {
        self.output_directory = path.as_ref().to_path_buf();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_config_sanitization() {
        let mut config = AppConfig {
            fps: 1000,
            quality_crf: 99,
            instant_replay_duration_secs: 900,
            ..Default::default()
        };
        config.sanitize();
        assert_eq!(config.fps, 240);
        assert_eq!(config.quality_crf, 51);
        assert_eq!(config.instant_replay_duration_secs, 300);
    }

    #[test]
    fn test_config_json_roundtrip() {
        let original = AppConfig::default();
        let json = original.to_json().unwrap();
        let parsed = AppConfig::from_json(&json).unwrap();
        assert_eq!(original, parsed);
    }
}
