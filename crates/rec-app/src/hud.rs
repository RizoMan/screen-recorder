//! Floating recording HUD widget and live visualizer.
//!
//! Conforms to `[SPEC-GUI-001]` and `RULE-I18N-001`.

use std::time::Duration;
use eframe::egui::{self, Color32, Vec2};
use rec_core::i18n::{text, I18nKey, Language};
use crate::theme::render_vu_meter;

/// Strongly-typed user action triggered from the floating recording HUD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudAction {
    /// Pause the active recording session.
    Pause,
    /// Resume a paused recording session.
    Resume,
    /// Stop and finalize the active recording session.
    Stop,
}

/// Live state for the compact floating recording HUD.
#[derive(Debug, Clone, PartialEq)]
pub struct FloatingHudState {
    /// True if recording is in progress.
    pub is_recording: bool,
    /// True if recording is paused.
    pub is_paused: bool,
    /// Elapsed duration.
    pub elapsed: Duration,
    /// Normalized left channel audio peak (0.0 to 1.0).
    pub audio_peak_l: f32,
    /// Normalized right channel audio peak (0.0 to 1.0).
    pub audio_peak_r: f32,
}

impl Default for FloatingHudState {
    fn default() -> Self {
        Self {
            is_recording: false,
            is_paused: false,
            elapsed: Duration::ZERO,
            audio_peak_l: 0.0,
            audio_peak_r: 0.0,
        }
    }
}

/// Formats a [`Duration`] into standard `MM:SS` or `HH:MM:SS` display format.
///
/// # Arguments
///
/// * `duration` - Elapsed time duration to format.
///
/// # Returns
///
/// Formatted time string (`"MM:SS"` when under 1 hour, `"HH:MM:SS"` otherwise).
#[must_use]
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Renders the floating HUD widget within an `egui::Ui` context.
///
/// # Arguments
///
/// * `ui` - Active `egui::Ui` region.
/// * `state` - Current [`FloatingHudState`] metrics.
/// * `lang` - Current display [`Language`].
///
/// # Returns
///
/// An optional [`HudAction`] triggered by user button clicks.
pub fn render_floating_hud(
    ui: &mut egui::Ui,
    state: &FloatingHudState,
    lang: Language,
) -> Option<HudAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        // Blinking recording dot or paused indicator
        let dot_color = if state.is_paused {
            Color32::from_rgb(234, 179, 8) // Amber/Yellow
        } else {
            Color32::from_rgb(239, 68, 68) // Bright Red
        };

        let (rect, _response) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.0, dot_color);

        // Timer text
        let time_str = format_duration(state.elapsed);
        ui.label(egui::RichText::new(time_str).strong().size(15.0));

        ui.separator();

        // Reuse centralized DRY VU meter widget from theme module
        let peak = state.audio_peak_l.max(state.audio_peak_r).clamp(0.0, 1.0);
        render_vu_meter(ui, peak, 40.0, 8.0);

        ui.separator();

        // Pause / Resume button
        if state.is_paused {
            if ui.button(text(I18nKey::ResumeRecording, lang)).clicked() {
                action = Some(HudAction::Resume);
            }
        } else if ui.button(text(I18nKey::PauseRecording, lang)).clicked() {
            action = Some(HudAction::Pause);
        }

        // Stop button
        if ui.button(text(I18nKey::StopRecording, lang)).clicked() {
            action = Some(HudAction::Stop);
        }
    });

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_secs(5)), "00:05");
        assert_eq!(format_duration(Duration::from_secs(65)), "01:05");
        assert_eq!(format_duration(Duration::from_secs(3665)), "01:01:05");
    }
}
