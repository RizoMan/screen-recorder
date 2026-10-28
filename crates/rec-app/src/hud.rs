//! Floating recording HUD widget and live visualizer.
//!
//! Conforms to `[SPEC-GUI-001]` and `RULE-I18N-001`.

use std::time::Duration;
use eframe::egui::{self, Color32, Vec2};
use rec_core::i18n::{text, I18nKey, Language};

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

/// Formats a duration into standard `HH:MM:SS` display format.
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

/// Renders the floating HUD widget within an egui UI context.
///
/// # Arguments
///
/// * `ui` - Active egui context.
/// * `state` - Current HUD metrics state.
/// * `lang` - Current display language.
///
/// # Returns
///
/// An optional action triggered by user button clicks (`"pause"`, `"resume"`, `"stop"`).
pub fn render_floating_hud(
    ui: &mut egui::Ui,
    state: &FloatingHudState,
    lang: Language,
) -> Option<&'static str> {
    let mut action = None;

    ui.horizontal(|ui| {
        // Blinking recording dot or paused icon
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

        // Audio VU Meter bar
        let vu_width = 40.0;
        let vu_height = 8.0;
        let (vu_rect, _) = ui.allocate_exact_size(Vec2::new(vu_width, vu_height), egui::Sense::hover());
        ui.painter().rect_filled(vu_rect, 2.0, Color32::from_rgb(30, 41, 59));

        let peak = state.audio_peak_l.max(state.audio_peak_r).clamp(0.0, 1.0);
        let filled_rect = egui::Rect::from_min_size(
            vu_rect.min,
            Vec2::new(vu_width * peak, vu_height),
        );
        let meter_color = if peak > 0.85 {
            Color32::from_rgb(239, 68, 68) // Red clip
        } else if peak > 0.6 {
            Color32::from_rgb(234, 179, 8) // Yellow
        } else {
            Color32::from_rgb(34, 197, 94) // Green
        };
        ui.painter().rect_filled(filled_rect, 2.0, meter_color);

        ui.separator();

        // Pause / Resume button
        if state.is_paused {
            if ui.button(text(I18nKey::ResumeRecording, lang)).clicked() {
                action = Some("resume");
            }
        } else if ui.button(text(I18nKey::PauseRecording, lang)).clicked() {
            action = Some("pause");
        }

        // Stop button
        if ui.button(text(I18nKey::StopRecording, lang)).clicked() {
            action = Some("stop");
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
