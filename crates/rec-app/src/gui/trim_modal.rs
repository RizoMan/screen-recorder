//! Interactive post-recording Quick Trim modal view and lossless export controller (`[SPEC-FEAT-TRIM-001]`).
//!
//! Applies the **Single Responsibility Principle (SRP)** by isolating the Quick Trim timeline UI
//! and delegating lossless RIFF stream-copy operations to [`rec_core::avi_trimmer`].

use std::fmt::Write;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, RichText, Stroke, Vec2};
use rec_core::avi_trimmer::{probe_avi_metadata, trim_avi_lossless, TrimRange};
use rec_core::i18n::{text, I18nKey, Language};

use crate::gui::models::{format_file_size, QuickTrimModalState, RecentMediaKind, RecentRecordingMeta};
use crate::gui::RecorderGuiApp;
use crate::theme::{
    elevated_frame, render_badge, BG_CARD, BG_ELEVATED, BORDER_SUBTLE, COLOR_PRIMARY,
    COLOR_PRIMARY_HOVER, COLOR_RUST_ORANGE, COLOR_SECONDARY, TEXT_BODY, TEXT_HEADLINE, TEXT_MUTED,
};

impl RecorderGuiApp {
    /// Opens the interactive Quick Trim modal for the most recently recorded `.avi` video clip (`[REQ-TRIM-001.4]`).
    pub fn open_quick_trim_modal(&mut self) {
        let Some(ref recent) = self.last_recording else {
            return;
        };
        if !recent.is_video() {
            return;
        }

        let source_path = recent.path.clone();
        match probe_avi_metadata(&source_path) {
            Ok(meta) => {
                let dur = meta.duration_secs.max(TrimRange::MIN_DURATION_SECS);
                self.trim_modal = Some(QuickTrimModalState {
                    source_path,
                    meta,
                    start_sec: 0.0,
                    end_sec: dur,
                });
            }
            Err(err) => {
                let mut msg = String::from("Quick Trim probe error: ");
                let _ = write!(&mut msg, "{}", err);
                self.status_message = Some(msg);
            }
        }
    }

    /// Executes the lossless AVI stream-copy trim for the active modal state and updates [`RecentRecordingMeta`].
    pub fn apply_quick_trim(&mut self) {
        let Some(state) = self.trim_modal.take() else {
            return;
        };

        let range = TrimRange::new(state.start_sec, state.end_sec, state.meta.duration_secs);
        let stem = state
            .source_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("Recording"));
        let mut trimmed_name = String::with_capacity(stem.len() + 16);
        let _ = write!(&mut trimmed_name, "{}_trimmed.avi", stem);
        let dst_path = state.source_path.with_file_name(trimmed_name);
        let lang = self.config.language;

        match trim_avi_lossless(&state.source_path, &dst_path, range) {
            Ok(saved_path) => {
                let trimmed_secs = range.trimmed_duration_secs().round() as u64;
                let mins = trimmed_secs / 60;
                let secs = trimmed_secs % 60;
                let mut duration_str = String::with_capacity(16);
                let _ = write!(&mut duration_str, "{:02}:{:02} min", mins, secs);

                let bytes = saved_path.metadata().map(|m| m.len()).unwrap_or(0);
                let size_str = format_file_size(bytes);

                self.last_recording = Some(RecentRecordingMeta {
                    path: saved_path,
                    kind: RecentMediaKind::Video { duration_str },
                    size_str,
                });
                self.status_message = Some(String::from(text(I18nKey::StatusTrimSaved, lang)));
            }
            Err(err) => {
                let mut msg = String::from("Quick Trim failed: ");
                let _ = write!(&mut msg, "{}", err);
                self.status_message = Some(msg);
            }
        }
    }

    /// Renders the interactive Quick Trim modal dialog when `self.trim_modal` is `Some` (`[REQ-TRIM-001.4]`).
    pub(crate) fn render_quick_trim_modal(&mut self, ctx: &egui::Context, lang: Language) {
        if self.trim_modal.is_none() {
            return;
        }

        let mut trigger_save = false;
        let mut trigger_close = false;

        egui::Window::new(text(I18nKey::TrimModalTitle, lang))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(520.0, 310.0))
            .frame(
                egui::Frame::NONE
                    .fill(BG_CARD)
                    .stroke(Stroke::new(1.5, COLOR_PRIMARY))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ctx, |ui| {
                let Some(ref mut state) = self.trim_modal else {
                    return;
                };

                let max_dur = state.meta.duration_secs.max(TrimRange::MIN_DURATION_SECS);

                ui.label(
                    RichText::new(text(I18nKey::TrimModalSubtitle, lang))
                        .size(12.0)
                        .color(TEXT_MUTED),
                );
                ui.add_space(10.0);

                // Source metadata badge row
                ui.horizontal(|ui| {
                    let file_label = state
                        .source_path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| String::from("Recording.avi"));
                    ui.label(
                        RichText::new(file_label)
                            .size(12.5)
                            .strong()
                            .color(TEXT_HEADLINE),
                    );

                    let mut spec_badge = String::with_capacity(48);
                    let _ = write!(
                        &mut spec_badge,
                        "{}x{} • {} FPS • {:.1}s",
                        state.meta.width, state.meta.height, state.meta.fps, max_dur
                    );
                    render_badge(
                        ui,
                        &spec_badge,
                        Color32::from_rgb(0x1E, 0x29, 0x3B),
                        COLOR_PRIMARY_HOVER,
                    );
                });

                ui.add_space(12.0);

                // Visual timeline bar highlighting the active [start_sec .. end_sec] slice
                let (bar_rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(bar_rect, CornerRadius::same(6), BG_ELEVATED);
                ui.painter().rect_stroke(
                    bar_rect,
                    CornerRadius::same(6),
                    Stroke::new(1.0, BORDER_SUBTLE),
                    egui::StrokeKind::Inside,
                );

                let start_ratio = (state.start_sec / max_dur).clamp(0.0, 1.0) as f32;
                let end_ratio = ((state.end_sec / max_dur) as f32).clamp(start_ratio, 1.0);
                let active_left = bar_rect.left() + bar_rect.width() * start_ratio;
                let active_right = bar_rect.left() + bar_rect.width() * end_ratio;
                let active_rect = Rect::from_min_max(
                    Pos2::new(active_left, bar_rect.top() + 2.0),
                    Pos2::new(active_right.max(active_left + 4.0), bar_rect.bottom() - 2.0),
                );
                ui.painter().rect_filled(
                    active_rect,
                    CornerRadius::same(4),
                    Color32::from_rgba_unmultiplied(0x3B, 0x82, 0xF6, 140),
                );

                ui.add_space(12.0);

                // In/Out Point Sliders inside elevated frame
                elevated_frame().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    let max_in = (state.end_sec - TrimRange::MIN_DURATION_SECS).max(0.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(text(I18nKey::TrimInPointLabel, lang))
                                .size(12.5)
                                .color(TEXT_BODY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Slider::new(&mut state.start_sec, 0.0..=max_in)
                                    .suffix(" s")
                                    .fixed_decimals(2),
                            );
                        });
                    });

                    ui.add_space(6.0);

                    let min_out = (state.start_sec + TrimRange::MIN_DURATION_SECS).min(max_dur);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(text(I18nKey::TrimOutPointLabel, lang))
                                .size(12.5)
                                .color(TEXT_BODY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Slider::new(&mut state.end_sec, min_out..=max_dur)
                                    .suffix(" s")
                                    .fixed_decimals(2),
                            );
                        });
                    });
                });

                ui.add_space(12.0);

                // Trimmed duration summary + Action buttons
                let trimmed_dur = (state.end_sec - state.start_sec).max(0.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(text(I18nKey::TrimSelectedDurationLabel, lang))
                            .size(12.0)
                            .color(TEXT_MUTED),
                    );
                    let mut dur_badge = String::with_capacity(24);
                    let _ = write!(&mut dur_badge, "{:.2} s", trimmed_dur);
                    render_badge(
                        ui,
                        &dur_badge,
                        Color32::from_rgb(0x06, 0x4E, 0x3B),
                        COLOR_SECONDARY,
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(text(I18nKey::BtnSaveTrim, lang))
                                        .size(12.5)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(COLOR_RUST_ORANGE)
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            trigger_save = true;
                        }

                        ui.add_space(6.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(text(I18nKey::BtnCancelTrim, lang))
                                        .size(12.0)
                                        .color(TEXT_BODY),
                                )
                                .fill(BG_ELEVATED)
                                .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            trigger_close = true;
                        }
                    });
                });
            });

        if trigger_save {
            self.apply_quick_trim();
        } else if trigger_close {
            self.trim_modal = None;
        }
    }
}
