//! Main Recording Dashboard view and header/footer navigation bars (`[SPEC-GUI-001]`).
//!
//! Applies the **Single Responsibility Principle (SRP)** and **DRY** by isolating dashboard
//! layout rendering and delegating all state mutations to [`RecorderGuiApp`] controller methods.

use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};
use rec_core::cursor_effects::CursorHighlightColor;
use rec_core::i18n::{text, I18nKey, Language};

use crate::gui::models::{
    default_monitor_label, format_monitor_label, AppTab, CaptureMode, ContainerFormat,
};
use crate::gui::RecorderGuiApp;
use crate::theme::{
    card_frame, elevated_frame, render_badge, render_vu_meter, BG_CARD, BG_ELEVATED, BORDER_SUBTLE,
    COLOR_PRIMARY, COLOR_PRIMARY_HOVER, COLOR_RUST_ORANGE, COLOR_SECONDARY, COLOR_TERTIARY,
    TEXT_BODY, TEXT_HEADLINE, TEXT_MUTED,
};

impl RecorderGuiApp {
    /// Renders the top navigation bar matching `main_screen.png`.
    pub(crate) fn render_top_header(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.horizontal(|ui| {
            // App icon: bullseye / target icon
            let (icon_rect, _) =
                ui.allocate_exact_size(Vec2::new(26.0, 26.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(icon_rect, CornerRadius::same(6), COLOR_PRIMARY);
            ui.painter()
                .circle_stroke(icon_rect.center(), 7.0, Stroke::new(2.0, Color32::WHITE));
            ui.painter()
                .circle_filled(icon_rect.center(), 3.0, Color32::WHITE);

            ui.add_space(6.0);

            ui.label(
                RichText::new("Screen Recorder")
                    .size(17.0)
                    .strong()
                    .color(TEXT_HEADLINE),
            );

            ui.add_space(4.0);

            render_badge(
                ui,
                text(I18nKey::ProBadge, lang),
                Color32::from_rgb(0x1E, 0x3A, 0x8A),
                COLOR_PRIMARY_HOVER,
            );

            let (status_text, status_bg, status_fg) = if self.pipeline.is_active() {
                (
                    format!("● {}", text(I18nKey::StatusRecording, lang)),
                    Color32::from_rgb(0x45, 0x1A, 0x1A),
                    COLOR_SECONDARY,
                )
            } else if self.pipeline.is_paused() {
                (
                    format!("● {}", text(I18nKey::StatusPaused, lang)),
                    Color32::from_rgb(0x45, 0x30, 0x10),
                    Color32::from_rgb(0xF5, 0x9E, 0x0B),
                )
            } else {
                (
                    format!("● {}", text(I18nKey::StatusIdle, lang)),
                    Color32::from_rgb(0x06, 0x4E, 0x3B),
                    COLOR_TERTIARY,
                )
            };
            render_badge(ui, &status_text, status_bg, status_fg);

            render_badge(
                ui,
                "Rust Core v2.4 (Sub-5ms)",
                Color32::from_rgb(0x2A, 0x18, 0x10),
                COLOR_RUST_ORANGE,
            );

            // Right side navigation and window controls
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let min_btn =
                    egui::Button::new(RichText::new("—").size(14.0).strong().color(TEXT_HEADLINE))
                        .min_size(Vec2::new(36.0, 32.0))
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(6));
                if ui
                    .add(min_btn)
                    .on_hover_text(text(I18nKey::TrayMinimizeToTray, lang))
                    .clicked()
                {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }

                ui.add_space(4.0);

                let lang_label = match lang {
                    Language::English => "🌐 EN",
                    Language::Spanish => "🌐 ES",
                };
                let lang_btn = egui::Button::new(
                    RichText::new(lang_label)
                        .size(13.0)
                        .strong()
                        .color(TEXT_HEADLINE),
                )
                .min_size(Vec2::new(76.0, 32.0))
                .fill(BG_CARD)
                .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(6));
                if ui.add(lang_btn).clicked() {
                    self.config.language = match lang {
                        Language::English => Language::Spanish,
                        Language::Spanish => Language::English,
                    };
                }

                ui.add_space(4.0);

                let is_settings = self.active_tab == AppTab::Settings;
                let settings_btn = egui::Button::new(
                    RichText::new(text(I18nKey::NavSettings, lang))
                        .size(13.0)
                        .strong()
                        .color(if is_settings {
                            Color32::WHITE
                        } else {
                            TEXT_BODY
                        }),
                )
                .min_size(Vec2::new(100.0, 32.0))
                .fill(if is_settings { COLOR_PRIMARY } else { BG_CARD })
                .stroke(Stroke::new(
                    1.0,
                    if is_settings {
                        COLOR_PRIMARY_HOVER
                    } else {
                        BORDER_SUBTLE
                    },
                ))
                .corner_radius(CornerRadius::same(6));

                if ui.add(settings_btn).clicked() {
                    self.active_tab = if is_settings {
                        AppTab::Dashboard
                    } else {
                        AppTab::Settings
                    };
                }

                ui.add_space(4.0);

                let is_general = self.active_tab == AppTab::Dashboard;
                let dash_btn = egui::Button::new(
                    RichText::new(text(I18nKey::NavDashboard, lang))
                        .size(13.0)
                        .strong()
                        .color(if is_general {
                            Color32::WHITE
                        } else {
                            TEXT_BODY
                        }),
                )
                .min_size(Vec2::new(90.0, 32.0))
                .fill(if is_general { COLOR_PRIMARY } else { BG_CARD })
                .stroke(Stroke::new(
                    1.0,
                    if is_general {
                        COLOR_PRIMARY_HOVER
                    } else {
                        BORDER_SUBTLE
                    },
                ))
                .corner_radius(CornerRadius::same(6));

                if ui.add(dash_btn).clicked() {
                    self.active_tab = AppTab::Dashboard;
                }
            });
        });
    }

    /// Renders the bottom status bar matching `main_screen.png`.
    pub(crate) fn render_bottom_status_bar(&self, ui: &mut egui::Ui, lang: Language) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("⚙ CPU: 1.4%    💾 RAM: 64 MB")
                    .size(12.0)
                    .color(TEXT_MUTED),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let status_str = if let Some(ref msg) = self.status_message {
                    msg.as_str()
                } else if self.pipeline.is_active() {
                    text(I18nKey::StatusRecording, lang)
                } else {
                    text(I18nKey::EngineReady, lang)
                };
                ui.label(RichText::new(status_str).size(12.0).color(TEXT_MUTED));
            });
        });
    }

    /// Renders the complete dashboard view conforming to `main_screen.png`.
    pub(crate) fn render_dashboard(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.add_space(4.0);

        // 1. Capture Mode Selection Bar
        ui.horizontal(|ui| {
            let modes = [
                (
                    CaptureMode::FullScreen,
                    format!("🖥 {}", text(I18nKey::ModeFullScreen, lang)),
                ),
                (
                    CaptureMode::Window,
                    format!("🗔 {}", text(I18nKey::ModeWindow, lang)),
                ),
                (
                    CaptureMode::CustomArea,
                    format!("⬚ {}", text(I18nKey::ModeFreeRegion, lang)),
                ),
                (
                    CaptureMode::Presets,
                    format!("⚙ {}", text(I18nKey::ModePresets, lang)),
                ),
            ];

            for (m, lbl) in modes {
                let selected = self.capture_mode == m;
                let btn = egui::Button::new(RichText::new(lbl).size(13.0).color(if selected {
                    Color32::WHITE
                } else {
                    TEXT_BODY
                }))
                .fill(if selected { COLOR_PRIMARY } else { BG_CARD })
                .stroke(Stroke::new(
                    1.0,
                    if selected {
                        COLOR_PRIMARY
                    } else {
                        BORDER_SUBTLE
                    },
                ))
                .corner_radius(CornerRadius::same(6));

                if ui.add(btn).clicked() {
                    self.capture_mode = m;
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                render_badge(
                    ui,
                    &format!("{:.2} FPS", self.config.fps),
                    Color32::from_rgb(0x06, 0x4E, 0x3B),
                    COLOR_TERTIARY,
                );

                let res_str: String = match self.monitors.get(self.selected_monitor_idx) {
                    Some(m) => format!("1080p ({}x{})", m.width, m.height),
                    None => "1080p (1920x1080)".to_string(),
                };
                render_badge(ui, &res_str, BG_ELEVATED, TEXT_BODY);
            });
        });

        ui.add_space(8.0);

        // 2. Primary Display & Target Card
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                let (mon_icon_rect, _) =
                    ui.allocate_exact_size(Vec2::new(38.0, 32.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(mon_icon_rect, CornerRadius::same(6), BG_ELEVATED);
                ui.painter().rect_stroke(
                    mon_icon_rect,
                    CornerRadius::same(6),
                    Stroke::new(1.0, COLOR_PRIMARY),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    mon_icon_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", self.selected_monitor_idx + 1),
                    egui::FontId::proportional(14.0),
                    COLOR_PRIMARY,
                );

                ui.add_space(8.0);

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(text(I18nKey::PrimaryDisplayLabel, lang))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );

                    // Use DRY helper format_monitor_label
                    let current_text = self
                        .monitors
                        .get(self.selected_monitor_idx)
                        .map(|m| format_monitor_label(m, lang))
                        .unwrap_or_else(|| default_monitor_label(lang));

                    egui::ComboBox::from_id_salt("display_selector")
                        .selected_text(RichText::new(current_text).size(13.0).color(TEXT_HEADLINE))
                        .width(420.0)
                        .show_ui(ui, |ui| {
                            for (idx, m) in self.monitors.iter().enumerate() {
                                let label = format_monitor_label(m, lang);
                                ui.selectable_value(&mut self.selected_monitor_idx, idx, label);
                            }
                        });
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    render_badge(
                        ui,
                        "Windows Graphics Capture (WGC)",
                        Color32::from_rgb(0x1E, 0x29, 0x3B),
                        COLOR_PRIMARY_HOVER,
                    );
                    ui.label(
                        RichText::new(text(I18nKey::CaptureBackendLabel, lang))
                            .size(12.0)
                            .color(TEXT_MUTED),
                    );
                });
            });
        });

        ui.add_space(8.0);

        // 3. Audio & Media Sources Row (3 Cards side by side)
        ui.columns(3, |columns| {
            // Column 1: System Audio
            card_frame().show(&mut columns[0], |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("🔊 {}", text(I18nKey::SystemAudioLabel, lang)))
                            .strong()
                            .color(TEXT_HEADLINE),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{}%", self.config.system_audio_volume_percent))
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                    });
                });
                ui.add_space(6.0);
                render_vu_meter(ui, self.hud_state.audio_peak_l, ui.available_width(), 6.0);
                ui.add_space(4.0);
                ui.label(
                    RichText::new("WASAPI System Loopback (48 kHz)")
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });

            // Column 2: Microphone (uses unified self.config.microphone_muted — DRY)
            card_frame().show(&mut columns[1], |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("🎙 {}", text(I18nKey::MicrophoneLabel, lang)))
                            .strong()
                            .color(TEXT_HEADLINE),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mute_icon = if self.config.microphone_muted {
                            "🔇"
                        } else {
                            "🎙"
                        };
                        if ui.button(mute_icon).clicked() {
                            self.config.microphone_muted = !self.config.microphone_muted;
                            self.pipeline.set_config(self.config.clone());
                        }
                    });
                });
                ui.add_space(6.0);
                let mic_peak = if self.config.microphone_muted {
                    0.0
                } else {
                    self.hud_state.audio_peak_r
                };
                render_vu_meter(ui, mic_peak, ui.available_width(), 6.0);
                ui.add_space(4.0);
                ui.label(
                    RichText::new("WASAPI Audio Input")
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });

            // Column 3: Smart Cursor Effects
            card_frame().show(&mut columns[2], |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("🖱 {}", text(I18nKey::CursorHaloToggle, lang)))
                            .strong()
                            .color(TEXT_HEADLINE),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .checkbox(&mut self.config.cursor_halo_enabled, "")
                            .changed()
                        {
                            self.pipeline.set_config(self.config.clone());
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("✨ {}", text(I18nKey::CursorRipplesToggle, lang)))
                            .size(11.0)
                            .color(TEXT_BODY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .checkbox(&mut self.config.cursor_click_ripples_enabled, "")
                            .changed()
                        {
                            self.pipeline.set_config(self.config.clone());
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let colors = [
                        (
                            CursorHighlightColor::YellowGlow,
                            "🟡",
                            text(I18nKey::CursorColorYellow, lang),
                        ),
                        (
                            CursorHighlightColor::ElectricBlue,
                            "🔵",
                            text(I18nKey::CursorColorBlue, lang),
                        ),
                        (
                            CursorHighlightColor::NeonEmerald,
                            "🟢",
                            text(I18nKey::CursorColorNeon, lang),
                        ),
                    ];
                    for (c, dot, tip) in colors {
                        let sel = self.config.cursor_halo_color == c;
                        if ui.selectable_label(sel, dot).on_hover_text(tip).clicked() {
                            self.config.cursor_halo_color = c;
                            self.pipeline.set_config(self.config.clone());
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        render_badge(
                            ui,
                            "D3D11",
                            Color32::from_rgb(0x06, 0x4E, 0x3B),
                            COLOR_TERTIARY,
                        );
                    });
                });
            });
        });

        ui.add_space(8.0);

        // 4. Output Destination Row
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("📁 {}", text(I18nKey::DestinationFolder, lang)))
                        .strong()
                        .color(TEXT_HEADLINE),
                );

                let path_str = if let Some(ref cf) = self.custom_output_file {
                    cf.display().to_string()
                } else {
                    self.config.output_directory.display().to_string()
                };

                elevated_frame().show(ui, |ui| {
                    ui.label(RichText::new(path_str).size(12.0).color(TEXT_BODY));
                });

                if ui.button(text(I18nKey::BtnChange, lang)).clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.config.set_output_directory(&folder);
                        self.pipeline.set_config(self.config.clone());
                    }
                }

                if ui
                    .button(format!("📂 {}", text(I18nKey::BtnOpenFolder, lang)))
                    .clicked()
                {
                    let _ = std::fs::create_dir_all(&self.config.output_directory);
                    let _ = open::that(&self.config.output_directory);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("fmt_sel")
                        .selected_text(match self.container_format {
                            ContainerFormat::Avi => "AVI ⌵",
                            ContainerFormat::Mp4 => "MP4 (Roadmap) ⌵",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.container_format,
                                ContainerFormat::Avi,
                                "AVI (MJPEG Lockstep)",
                            );
                            ui.selectable_value(
                                &mut self.container_format,
                                ContainerFormat::Mp4,
                                "MP4 (H.264 - Roadmap)",
                            );
                        });
                });
            });
        });

        if self.container_format == ContainerFormat::Mp4 {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(text(I18nKey::Mp4RoadmapNotice, lang))
                        .size(11.0)
                        .color(Color32::from_rgb(0x38, 0xBD, 0xF8)),
                );
            });
            ui.add_space(4.0);
        }

        ui.add_space(10.0);

        // 5. Giant Action Buttons Row (delegates to unified DRY toggle_recording & trigger_screenshot)
        ui.horizontal(|ui| {
            let is_rec = self.pipeline.is_active();
            let rec_label = if is_rec {
                format!("■ {} [F9]", text(I18nKey::BtnStopRecordingF9, lang))
            } else {
                format!("● {} [F9]", text(I18nKey::BtnStartRecordingF9, lang))
            };

            let rec_btn = egui::Button::new(
                RichText::new(rec_label)
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(if is_rec {
                Color32::from_rgb(0x99, 0x1B, 0x1B)
            } else {
                COLOR_SECONDARY
            })
            .min_size(Vec2::new(ui.available_width() - 220.0, 48.0))
            .corner_radius(CornerRadius::same(8));

            if ui.add(rec_btn).clicked() {
                self.toggle_recording();
            }

            ui.add_space(8.0);

            let snap_label = format!("📷 {} [F10]", text(I18nKey::BtnCaptureImageF10, lang));
            let snap_btn = egui::Button::new(
                RichText::new(snap_label)
                    .size(14.0)
                    .strong()
                    .color(TEXT_HEADLINE),
            )
            .fill(BG_CARD)
            .stroke(Stroke::new(1.0, BORDER_SUBTLE))
            .min_size(Vec2::new(210.0, 48.0))
            .corner_radius(CornerRadius::same(8));

            if ui.add(snap_btn).clicked() {
                self.trigger_screenshot();
            }
        });

        ui.add_space(10.0);

        // 6. Recent Recording Card
        self.render_recent_recording_card(ui, lang);
    }

    /// Renders the bottom card displaying metadata and quick actions for the most recent capture.
    fn render_recent_recording_card(&mut self, ui: &mut egui::Ui, lang: Language) {
        let mut should_delete = false;
        let mut should_copy = false;
        let mut should_play = false;
        let mut should_open_folder = false;
        let mut should_export_gif = false;

        if let Some(ref meta) = self.last_recording {
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let is_snapshot = meta.is_snapshot();
                    let is_gif = meta.is_gif();
                    let is_video = meta.is_video();
                    let (icon_rect, _) =
                        ui.allocate_exact_size(Vec2::new(36.0, 36.0), egui::Sense::hover());
                    let bg_color = if is_snapshot {
                        Color32::from_rgb(0x1E, 0x3A, 0x8A)
                    } else if is_gif {
                        Color32::from_rgb(0x4C, 0x1D, 0x95)
                    } else {
                        Color32::from_rgb(0x06, 0x4E, 0x3B)
                    };
                    ui.painter()
                        .rect_filled(icon_rect, CornerRadius::same(8), bg_color);
                    let icon_glyph = if is_snapshot {
                        "📷"
                    } else if is_gif {
                        "🎞"
                    } else {
                        "🎬"
                    };
                    ui.painter().text(
                        icon_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        icon_glyph,
                        egui::FontId::proportional(16.0),
                        Color32::WHITE,
                    );

                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let file_name =
                                meta.path.file_name().unwrap_or_default().to_string_lossy();
                            ui.label(RichText::new(file_name).strong().color(TEXT_HEADLINE));
                            let badge_text = if is_snapshot {
                                text(I18nKey::SnapshotBadge, lang)
                            } else if is_gif {
                                text(I18nKey::GifBadge, lang)
                            } else {
                                text(I18nKey::RecentRecording, lang)
                            };
                            render_badge(ui, badge_text, bg_color, COLOR_TERTIARY);
                        });

                        ui.label(
                            RichText::new(format!(
                                "⏱ {}   •   💾 {}",
                                meta.display_duration(lang),
                                meta.size_str
                            ))
                            .size(11.0)
                            .color(TEXT_MUTED),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let del_btn = egui::Button::new(
                            RichText::new("🗑")
                                .size(12.0)
                                .color(Color32::from_rgb(0xEF, 0x44, 0x44)),
                        )
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(0x7F, 0x1D, 0x1D)))
                        .corner_radius(CornerRadius::same(6));
                        if ui
                            .add(del_btn)
                            .on_hover_text(text(I18nKey::BtnDelete, lang))
                            .clicked()
                        {
                            should_delete = true;
                        }

                        let folder_btn = egui::Button::new(
                            RichText::new(format!("📂 {}", text(I18nKey::BtnShowInFolder, lang)))
                                .size(12.0)
                                .color(TEXT_HEADLINE),
                        )
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(6));
                        if ui.add(folder_btn).clicked() {
                            should_open_folder = true;
                        }

                        let copy_btn = egui::Button::new(
                            RichText::new(format!("📋 {}", text(I18nKey::BtnCopyPath, lang)))
                                .size(12.0)
                                .color(TEXT_HEADLINE),
                        )
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(6));
                        if ui.add(copy_btn).clicked() {
                            should_copy = true;
                        }

                        if is_video {
                            let gif_btn = egui::Button::new(
                                RichText::new(format!("🎞 {}", text(I18nKey::BtnExportGif, lang)))
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(0x6D, 0x28, 0xD9))
                            .corner_radius(CornerRadius::same(6));
                            if ui.add(gif_btn).clicked() {
                                should_export_gif = true;
                            }
                        }

                        let play_btn = egui::Button::new(
                            RichText::new(format!("▶ {}", text(I18nKey::BtnPlay, lang)))
                                .size(12.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(COLOR_PRIMARY)
                        .corner_radius(CornerRadius::same(6));
                        if ui.add(play_btn).clicked() {
                            should_play = true;
                        }
                    });
                });
            });
        }

        if should_export_gif {
            self.export_last_recording_to_gif();
        }

        if should_delete {
            if let Some(ref meta) = self.last_recording {
                let _ = std::fs::remove_file(&meta.path);
            }
            self.last_recording = None;
            self.status_message = Some(text(I18nKey::StatusFileDeleted, lang).to_string());
        }

        if should_copy {
            if let Some(ref meta) = self.last_recording {
                ui.ctx().copy_text(meta.path.to_string_lossy().to_string());
                self.status_message =
                    Some(text(I18nKey::StatusCopiedToClipboard, lang).to_string());
            }
        }

        if should_play {
            if let Some(ref meta) = self.last_recording {
                let _ = open::that(&meta.path);
            }
        }

        if should_open_folder {
            if let Some(ref meta) = self.last_recording {
                if let Some(parent) = meta.path.parent() {
                    let _ = open::that(parent);
                }
            }
        }
    }
}
