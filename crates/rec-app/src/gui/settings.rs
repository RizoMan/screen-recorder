//! Settings Modal view and categorized configuration panels (`[SPEC-GUI-001]`).
//!
//! Applies the **Single Responsibility Principle (SRP)** by isolating the five settings
//! sub-panels (`VideoEncoder`, `AudioChannels`, `OutputFormat`, `Hotkeys`, `PerfBuffer`)
//! into a dedicated presentation module.

use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};
use rec_core::config::AppConfig;
use rec_core::cursor_effects::CursorHighlightColor;
use rec_core::gif_exporter::GifScale;
use rec_core::i18n::{text, I18nKey, Language};

use crate::gui::models::{AppTab, EncoderChoice, RateControlMode, SettingsCategory};
use crate::gui::RecorderGuiApp;
use crate::theme::{
    card_frame, elevated_frame, render_badge, render_vu_meter, BG_CARD, BG_CARD_HOVER, BG_ELEVATED,
    BORDER_SUBTLE, COLOR_PRIMARY, COLOR_PRIMARY_HOVER, COLOR_SECONDARY, COLOR_TERTIARY, TEXT_BODY,
    TEXT_HEADLINE, TEXT_MUTED,
};

impl RecorderGuiApp {
    /// Renders the complete settings view conforming to `settings_screen_1.png`.
    pub(crate) fn render_settings_modal(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(text(I18nKey::SettingsEncoderTitle, lang))
                    .size(16.0)
                    .strong()
                    .color(TEXT_HEADLINE),
            );
            render_badge(ui, "NVENC / Rust-AV Core v2.4", BG_ELEVATED, TEXT_BODY);
            render_badge(
                ui,
                text(I18nKey::HwReadyPill, lang),
                Color32::from_rgb(0x06, 0x4E, 0x3B),
                COLOR_TERTIARY,
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(text(I18nKey::BtnCloseModal, lang)).clicked() {
                    self.active_tab = AppTab::Dashboard;
                }
            });
        });

        ui.add_space(10.0);

        // Two-column layout: Left Sidebar + Main Settings Content
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(190.0);
                let cats = [
                    (
                        SettingsCategory::VideoEncoder,
                        format!("🎞 {}", text(I18nKey::SettingsTabVideoEncoder, lang)),
                    ),
                    (
                        SettingsCategory::AudioChannels,
                        format!("🔊 {}", text(I18nKey::SettingsTabAudioChannels, lang)),
                    ),
                    (
                        SettingsCategory::OutputFormat,
                        format!("📁 {}", text(I18nKey::SettingsTabOutputFormat, lang)),
                    ),
                    (
                        SettingsCategory::Hotkeys,
                        format!("⌨ {}", text(I18nKey::SettingsTabHotkeys, lang)),
                    ),
                    (
                        SettingsCategory::PerfBuffer,
                        format!("⚡ {}", text(I18nKey::SettingsTabPerfBuffer, lang)),
                    ),
                ];

                for (cat, label) in cats {
                    let is_active = self.settings_cat == cat;
                    let btn = egui::Button::new(
                        RichText::new(label)
                            .size(13.0)
                            .color(if is_active { Color32::WHITE } else { TEXT_BODY }),
                    )
                    .fill(if is_active { BG_CARD_HOVER } else { BG_CARD })
                    .stroke(Stroke::new(
                        1.0,
                        if is_active { COLOR_PRIMARY } else { BORDER_SUBTLE },
                    ))
                    .min_size(Vec2::new(186.0, 36.0))
                    .corner_radius(CornerRadius::same(6));

                    if ui.add(btn).clicked() {
                        self.settings_cat = cat;
                    }
                    ui.add_space(4.0);
                }

                ui.add_space(16.0);

                // Sidebar telemetry card
                card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(text(I18nKey::LatencyMetric, lang))
                                .size(10.0)
                                .color(TEXT_MUTED),
                        );
                        ui.label(
                            RichText::new("2.8 ms")
                                .size(10.0)
                                .strong()
                                .color(COLOR_TERTIARY),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(text(I18nKey::DroppedFramesMetric, lang))
                                .size(10.0)
                                .color(TEXT_MUTED),
                        );
                        ui.label(RichText::new("0 (0.00%)").size(10.0).color(TEXT_BODY));
                    });
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(text(I18nKey::VramMetric, lang))
                                .size(10.0)
                                .color(TEXT_MUTED),
                        );
                        ui.label(
                            RichText::new("64 MB")
                                .size(10.0)
                                .color(COLOR_PRIMARY_HOVER),
                        );
                    });
                });
            });

            ui.separator();

            // Right Main Settings Pane
            ui.vertical(|ui| match self.settings_cat {
                SettingsCategory::VideoEncoder => self.render_settings_video_encoder(ui, lang),
                SettingsCategory::AudioChannels => self.render_settings_audio(ui, lang),
                SettingsCategory::OutputFormat => self.render_settings_output(ui, lang),
                SettingsCategory::Hotkeys => self.render_settings_hotkeys(ui, lang),
                SettingsCategory::PerfBuffer => self.render_settings_perf(ui, lang),
            });
        });

        // Bottom Footer
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui
                .button(format!("↺ {}", text(I18nKey::BtnResetDefaults, lang)))
                .clicked()
            {
                self.config = AppConfig::new();
                self.pipeline.set_config(self.config.clone());
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save_btn = egui::Button::new(
                    RichText::new(format!("✓ {}", text(I18nKey::BtnSave, lang)))
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(COLOR_PRIMARY)
                .corner_radius(CornerRadius::same(6));

                if ui.add(save_btn).clicked() {
                    self.pipeline.set_config(self.config.clone());
                    self.active_tab = AppTab::Dashboard;
                }

                if ui.button(text(I18nKey::BtnCancel, lang)).clicked() {
                    self.active_tab = AppTab::Dashboard;
                }
            });
        });
    }

    /// Renders the Video & Encoder tab.
    fn render_settings_video_encoder(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("⚙ {}", text(I18nKey::HwEncoderBackendSection, lang)))
                    .strong()
                    .color(TEXT_HEADLINE),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                render_badge(
                    ui,
                    "Direct HW Accel",
                    Color32::from_rgb(0x06, 0x4E, 0x3B),
                    COLOR_TERTIARY,
                );
            });
        });

        ui.add_space(6.0);

        ui.columns(2, |cols| {
            let is_nvenc = self.encoder_choice == EncoderChoice::NvidiaNvenc;
            card_frame().show(&mut cols[0], |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(is_nvenc, "✓ NVIDIA NVENC").clicked() {
                        self.encoder_choice = EncoderChoice::NvidiaNvenc;
                    }
                    render_badge(
                        ui,
                        text(I18nKey::RecommendedBadge, lang),
                        Color32::from_rgb(0x1E, 0x3A, 0x8A),
                        COLOR_PRIMARY_HOVER,
                    );
                });
                ui.label(
                    RichText::new("H.264 / HEVC / AV1 via Dual NVENC Gen 8")
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });

            let is_amf = self.encoder_choice == EncoderChoice::AmdAmf;
            card_frame().show(&mut cols[1], |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(is_amf, "AMD AMF Core").clicked() {
                        self.encoder_choice = EncoderChoice::AmdAmf;
                    }
                    render_badge(ui, "VCN 4.0", BG_ELEVATED, TEXT_MUTED);
                });
                ui.label(
                    RichText::new("Radeon RDNA3 Hardware Encoder")
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });
        });

        ui.add_space(4.0);

        ui.columns(2, |cols| {
            let is_qsv = self.encoder_choice == EncoderChoice::IntelQuickSync;
            card_frame().show(&mut cols[0], |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(is_qsv, "Intel QuickSync").clicked() {
                        self.encoder_choice = EncoderChoice::IntelQuickSync;
                    }
                    render_badge(ui, "QSV Gen12", BG_ELEVATED, TEXT_MUTED);
                });
                ui.label(
                    RichText::new("Iris Xe & Arc Discrete Hardware Encoder")
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });

            let is_cpu = self.encoder_choice == EncoderChoice::SoftwareCpu;
            card_frame().show(&mut cols[1], |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(is_cpu, "Software MJPEG / x264").clicked() {
                        self.encoder_choice = EncoderChoice::SoftwareCpu;
                    }
                    render_badge(ui, "CPU RUST", BG_ELEVATED, TEXT_MUTED);
                });
                ui.label(
                    RichText::new("Multi-threaded Pure Rust Encoder")
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });
        });

        ui.add_space(6.0);

        elevated_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("🖥 {}", text(I18nKey::DetectedGpuLabel, lang)))
                        .strong()
                        .color(TEXT_HEADLINE),
                );
                ui.label(
                    RichText::new("Direct3D 11 Hardware Compositor • Zero-Copy WGC")
                        .color(COLOR_PRIMARY_HOVER),
                );
            });
        });

        ui.add_space(10.0);

        ui.label(
            RichText::new(format!("📊 {}", text(I18nKey::BitrateSectionTitle, lang)))
                .strong()
                .color(TEXT_HEADLINE),
        );
        ui.add_space(4.0);

        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::BitrateControlMode, lang)).color(TEXT_HEADLINE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let modes = [
                        (RateControlMode::Cqp, "CQP / CQ"),
                        (RateControlMode::Vbr, "VBR"),
                        (RateControlMode::Cbr, "CBR"),
                    ];
                    for (m, lbl) in modes {
                        let selected = self.rate_control == m;
                        if ui.selectable_label(selected, lbl).clicked() {
                            self.rate_control = m;
                        }
                    }
                });
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::TargetBitrateLabel, lang)).color(TEXT_HEADLINE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mbps = self.target_bitrate_kbps as f64 / 1000.0;
                    ui.label(
                        RichText::new(format!("{} Kbps ({:.1} Mbps)", self.target_bitrate_kbps, mbps))
                            .strong()
                            .color(COLOR_PRIMARY_HOVER),
                    );
                });
            });

            ui.add(
                egui::Slider::new(&mut self.target_bitrate_kbps, 2_000..=50_000).show_value(false),
            );

            ui.horizontal(|ui| {
                ui.label(RichText::new("2,500k (720p)").size(10.0).color(TEXT_MUTED));
                ui.add_space(40.0);
                ui.label(RichText::new("6,000k (1080p30)").size(10.0).color(TEXT_MUTED));
                ui.add_space(40.0);
                ui.label(
                    RichText::new("12,000k (1080p60 Pro)")
                        .size(10.0)
                        .strong()
                        .color(COLOR_PRIMARY_HOVER),
                );
                ui.add_space(40.0);
                ui.label(RichText::new("24,000k (2K)").size(10.0).color(TEXT_MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("50,000k (4K60 HDR)").size(10.0).color(TEXT_MUTED));
                });
            });

            ui.add_space(8.0);

            ui.columns(3, |cols| {
                cols[0].vertical(|ui| {
                    ui.label(
                        RichText::new(text(I18nKey::NvencPresetLabel, lang))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                    ui.horizontal(|ui| {
                        let presets = [(1, "P1"), (4, "P4"), (7, "P7")];
                        for (p, lbl) in presets {
                            if ui.selectable_label(self.nvenc_preset == p, lbl).clicked() {
                                self.nvenc_preset = p;
                            }
                        }
                    });
                });

                cols[1].vertical(|ui| {
                    ui.label(
                        RichText::new(text(I18nKey::ProfileLabel, lang))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                    elevated_frame().show(ui, |ui| {
                        ui.label(RichText::new("High (AVC/H.264)").size(12.0).color(TEXT_HEADLINE));
                    });
                });

                cols[2].vertical(|ui| {
                    ui.label(
                        RichText::new(text(I18nKey::KeyframesLabel, lang))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                    elevated_frame().show(ui, |ui| {
                        ui.label(RichText::new("2s (120 frames)").size(12.0).color(TEXT_HEADLINE));
                    });
                });
            });
        });
    }

    /// Renders the Audio & Channels tab.
    fn render_settings_audio(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(
            RichText::new(text(I18nKey::SettingsTabAudioChannels, lang))
                .size(16.0)
                .strong()
                .color(TEXT_HEADLINE),
        );
        ui.add_space(8.0);

        let mut config_changed = false;

        // 1. System Audio Card (WASAPI Loopback)
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .checkbox(
                        &mut self.config.record_system_audio,
                        text(I18nKey::SystemAudioLabel, lang),
                    )
                    .changed()
                {
                    config_changed = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{}%", self.config.system_audio_volume_percent))
                            .strong()
                            .color(if self.config.record_system_audio {
                                COLOR_PRIMARY_HOVER
                            } else {
                                TEXT_MUTED
                            }),
                    );
                });
            });

            ui.add_space(6.0);
            ui.label(
                RichText::new(text(I18nKey::SystemAudioVolumeLabel, lang))
                    .size(11.0)
                    .color(TEXT_MUTED),
            );
            let mut sys_vol = self.config.system_audio_volume_percent;
            ui.add_enabled_ui(self.config.record_system_audio, |ui| {
                if ui
                    .add(egui::Slider::new(&mut sys_vol, 0..=150).suffix("%"))
                    .changed()
                {
                    self.config.system_audio_volume_percent = sys_vol;
                    config_changed = true;
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for &preset in &[50, 100, 150] {
                    if ui
                        .selectable_label(
                            self.config.system_audio_volume_percent == preset,
                            format!("{}%", preset),
                        )
                        .clicked()
                    {
                        self.config.system_audio_volume_percent = preset;
                        config_changed = true;
                    }
                }
            });
        });

        ui.add_space(10.0);

        // 2. Microphone Input & Gain Card
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .checkbox(
                        &mut self.config.record_microphone,
                        text(I18nKey::MicrophoneLabel, lang),
                    )
                    .changed()
                {
                    config_changed = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (mute_icon_text, text_color) = if self.config.microphone_muted {
                        (
                            format!("🔇 {}", text(I18nKey::BtnUnmute, lang)),
                            COLOR_SECONDARY,
                        )
                    } else {
                        (format!("🎙 {}", text(I18nKey::BtnMute, lang)), TEXT_HEADLINE)
                    };
                    if ui
                        .button(RichText::new(mute_icon_text).color(text_color))
                        .clicked()
                    {
                        self.config.microphone_muted = !self.config.microphone_muted;
                        config_changed = true;
                    }
                });
            });

            ui.add_space(6.0);
            ui.label(
                RichText::new(text(I18nKey::MicrophoneGainLabel, lang))
                    .size(11.0)
                    .color(TEXT_MUTED),
            );
            let mut mic_gain = self.config.microphone_gain_percent;
            ui.add_enabled_ui(
                self.config.record_microphone && !self.config.microphone_muted,
                |ui| {
                    if ui
                        .add(egui::Slider::new(&mut mic_gain, 0..=200).suffix("%"))
                        .changed()
                    {
                        self.config.microphone_gain_percent = mic_gain;
                        config_changed = true;
                    }
                },
            );

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for &preset in &[50, 100, 150, 200] {
                    if ui
                        .selectable_label(
                            self.config.microphone_gain_percent == preset,
                            format!("{}%", preset),
                        )
                        .clicked()
                    {
                        self.config.microphone_gain_percent = preset;
                        config_changed = true;
                    }
                }
            });
        });

        ui.add_space(10.0);

        // 3. Live Audio VU Meter & Technical Specifications Card
        card_frame().show(ui, |ui| {
            let (peak_l, peak_r) = self.pipeline.live_audio_peaks();
            let peak_avg = ((peak_l + peak_r) * 0.5).clamp(0.0, 1.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(text(I18nKey::AudioSampleRateLabel, lang))
                        .strong()
                        .color(TEXT_HEADLINE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    render_vu_meter(ui, peak_avg, 100.0, 8.0);
                    ui.label(
                        RichText::new(text(I18nKey::HudAudioLevel, lang))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                });
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new(text(I18nKey::AudioSpecsDescription, lang))
                    .color(TEXT_MUTED)
                    .size(12.0),
            );
        });

        if config_changed {
            self.pipeline.set_config(self.config.clone());
        }
    }

    /// Renders the Output & Format tab.
    fn render_settings_output(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(
            RichText::new(text(I18nKey::SettingsTabOutputFormat, lang))
                .size(16.0)
                .strong()
                .color(TEXT_HEADLINE),
        );
        ui.add_space(8.0);

        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(text(I18nKey::OutputDirectory, lang))
                    .strong()
                    .color(TEXT_HEADLINE),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                elevated_frame().show(ui, |ui| {
                    ui.label(self.config.output_directory.display().to_string());
                });
                if ui.button(text(I18nKey::BtnChange, lang)).clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.config.set_output_directory(&folder);
                    }
                }
            });
        });

        ui.add_space(10.0);

        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(text(I18nKey::UiScaleLabel, lang))
                        .strong()
                        .color(TEXT_HEADLINE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{}%", self.config.font_scale_percent))
                            .strong()
                            .color(COLOR_PRIMARY_HOVER),
                    );
                });
            });

            ui.add_space(6.0);
            let mut scale = self.config.font_scale_percent;
            if ui
                .add(egui::Slider::new(&mut scale, 80..=150).suffix("%"))
                .changed()
            {
                self.config.font_scale_percent = scale;
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let presets = [
                    (85, text(I18nKey::UiScaleSmall, lang)),
                    (100, text(I18nKey::UiScaleNormal, lang)),
                    (115, text(I18nKey::UiScaleLarge, lang)),
                    (130, text(I18nKey::UiScaleExtraLarge, lang)),
                ];
                for (val, lbl) in presets {
                    let is_sel = self.config.font_scale_percent == val;
                    let btn = egui::Button::new(
                        RichText::new(lbl)
                            .size(12.0)
                            .color(if is_sel { Color32::WHITE } else { TEXT_BODY }),
                    )
                    .fill(if is_sel { COLOR_PRIMARY } else { BG_ELEVATED })
                    .corner_radius(CornerRadius::same(6));
                    if ui.add(btn).clicked() {
                        self.config.font_scale_percent = val;
                    }
                }
            });
        });

        ui.add_space(10.0);

        // Animated GIF HD Export Configuration Card ([SPEC-FEAT-GIF-001])
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(format!("🎞 {}", text(I18nKey::GifExportSectionTitle, lang)))
                    .strong()
                    .color(TEXT_HEADLINE),
            );
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::GifScaleLabel, lang)).color(TEXT_BODY));
                let scales = [
                    (GifScale::Scale50, "50% (Compact)"),
                    (GifScale::Scale75, "75% (HD)"),
                    (GifScale::Scale100, "100% (Full)"),
                ];
                for (scale_opt, lbl) in scales {
                    if ui
                        .selectable_label(self.config.gif_scale == scale_opt, lbl)
                        .clicked()
                    {
                        self.config.gif_scale = scale_opt;
                    }
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::GifFpsLabel, lang)).color(TEXT_BODY));
                for &fps_opt in &[10u32, 15, 30] {
                    if ui
                        .selectable_label(
                            self.config.gif_fps == fps_opt,
                            format!("{} FPS", fps_opt),
                        )
                        .clicked()
                    {
                        self.config.gif_fps = fps_opt;
                    }
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::GifMaxDurationLabel, lang)).color(TEXT_BODY));
                ui.add(
                    egui::Slider::new(&mut self.config.gif_max_duration_secs, 5..=15).suffix(" s"),
                );
            });
        });
    }

    /// Renders the Keyboard Hotkeys tab.
    fn render_settings_hotkeys(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(
            RichText::new(text(I18nKey::SettingsTabHotkeys, lang))
                .size(16.0)
                .strong()
                .color(TEXT_HEADLINE),
        );
        ui.add_space(8.0);

        card_frame().show(ui, |ui| {
            let hotkeys = [
                (text(I18nKey::HotkeyStartStopLabel, lang), "F9"),
                (text(I18nKey::HotkeySnapshotLabel, lang), "F10"),
                (text(I18nKey::HotkeyReplayLabel, lang), "F11"),
                (text(I18nKey::HotkeyPauseResumeLabel, lang), "F8"),
            ];

            for (desc, key) in hotkeys {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(desc).color(TEXT_BODY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        render_badge(ui, key, COLOR_PRIMARY, Color32::WHITE);
                    });
                });
                ui.add_space(4.0);
            }
        });
    }

    /// Renders the Performance & Buffer tab.
    fn render_settings_perf(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(
            RichText::new(text(I18nKey::SettingsTabPerfBuffer, lang))
                .size(16.0)
                .strong()
                .color(TEXT_HEADLINE),
        );
        ui.add_space(8.0);

        let mut changed = false;

        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(format!("🖱 {}", text(I18nKey::CursorEffectsSection, lang)))
                    .strong()
                    .color(TEXT_HEADLINE),
            );
            ui.add_space(6.0);

            if ui
                .checkbox(
                    &mut self.config.cursor_halo_enabled,
                    text(I18nKey::CursorHaloToggle, lang),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .checkbox(
                    &mut self.config.cursor_click_ripples_enabled,
                    text(I18nKey::CursorRipplesToggle, lang),
                )
                .changed()
            {
                changed = true;
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::CursorHaloRadiusLabel, lang)).color(TEXT_BODY));
                if ui
                    .add(egui::Slider::new(&mut self.config.cursor_halo_radius, 12..=64).suffix(" px"))
                    .changed()
                {
                    changed = true;
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::CursorColorLabel, lang)).color(TEXT_BODY));
                let presets = [
                    (
                        CursorHighlightColor::YellowGlow,
                        format!("🟡 {}", text(I18nKey::CursorColorYellow, lang)),
                    ),
                    (
                        CursorHighlightColor::ElectricBlue,
                        format!("🔵 {}", text(I18nKey::CursorColorBlue, lang)),
                    ),
                    (
                        CursorHighlightColor::NeonEmerald,
                        format!("🟢 {}", text(I18nKey::CursorColorNeon, lang)),
                    ),
                ];
                for (color, lbl) in presets {
                    if ui
                        .selectable_label(self.config.cursor_halo_color == color, lbl)
                        .clicked()
                    {
                        self.config.cursor_halo_color = color;
                        changed = true;
                    }
                }
            });
        });

        ui.add_space(10.0);

        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(text(I18nKey::InstantReplaySave, lang))
                    .strong()
                    .color(TEXT_HEADLINE),
            );
            if ui
                .add(egui::Slider::new(&mut self.config.instant_replay_duration_secs, 5..=120).suffix(" s"))
                .changed()
            {
                changed = true;
            }
            ui.add_space(8.0);
            if ui
                .checkbox(
                    &mut self.config.use_hardware_acceleration,
                    text(I18nKey::HardwareAcceleration, lang),
                )
                .changed()
            {
                changed = true;
            }
        });

        if changed {
            self.pipeline.set_config(self.config.clone());
        }
    }
}

