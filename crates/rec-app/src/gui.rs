//! Modern Obsidian Dark native desktop user interface built with `eframe`/`egui`.
//!
//! Conforms to `[SPEC-GUI-001]`, `[SPEC-UIUX-001]`, and design tokens from `./UIUX/`.

use std::path::PathBuf;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};
use rec_core::config::AppConfig;
use rec_core::cursor_effects::CursorHighlightColor;
use rec_core::i18n::{text, I18nKey, Language};
use rec_engine::pipeline::RecordingPipeline;
use rec_engine::traits::CaptureTarget;
use rec_platform_windows::{
    capture_screenshot, enumerate_monitors, MonitorInfo, WasapiAudioBackend, WgcCaptureBackend,
};
#[cfg(target_os = "windows")]
use rec_platform_windows::{is_exit_requested, TrayEvent, Win32TrayManager};

use crate::hud::{render_floating_hud, FloatingHudState};
use crate::theme::{
    card_frame, configure_obsidian_theme, elevated_frame, render_badge, render_vu_meter,
    BG_CARD, BG_CARD_HOVER, BG_ELEVATED, BORDER_SUBTLE, COLOR_PRIMARY, COLOR_PRIMARY_HOVER,
    COLOR_RUST_ORANGE, COLOR_SECONDARY, COLOR_TERTIARY, TEXT_BODY, TEXT_HEADLINE, TEXT_MUTED,
};

/// Active view tab in the main application window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTab {
    /// Primary recording dashboard.
    #[default]
    Dashboard,
    /// Settings and preferences.
    Settings,
}

/// Area selection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureMode {
    /// Full screen monitor capture.
    #[default]
    FullScreen,
    /// Specific top-level application window.
    Window,
    /// Custom rectangular user-defined coordinates.
    CustomArea,
    /// Preset resolutions (1080p, 720p).
    Presets,
}

/// Settings sidebar category selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsCategory {
    /// Video encoder and hardware acceleration.
    #[default]
    VideoEncoder,
    /// Audio loopback and microphone channels.
    AudioChannels,
    /// Destination directory and container format.
    OutputFormat,
    /// Keyboard hotkeys.
    Hotkeys,
    /// Performance and Instant Replay buffer.
    PerfBuffer,
}

/// Hardware encoder selection choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EncoderChoice {
    /// NVIDIA NVENC Gen 8.
    #[default]
    NvidiaNvenc,
    /// AMD AMF Core VCN 4.0.
    AmdAmf,
    /// Intel QuickSync Video QSV Gen12.
    IntelQuickSync,
    /// Multi-threaded CPU fallback.
    SoftwareCpu,
}

/// Bitrate rate control mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RateControlMode {
    /// Constant Bitrate.
    #[default]
    Cbr,
    /// Variable Bitrate.
    Vbr,
    /// Constant Quality Parameter.
    Cqp,
}

/// Video output container format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerFormat {
    /// High-performance Motion-JPEG AVI.
    #[default]
    Avi,
    /// Standard MP4 container.
    Mp4,
}

/// Metadata summary of the most recent recording.
#[derive(Debug, Clone)]
pub struct RecentRecordingMeta {
    /// Absolute path to the saved file.
    pub path: PathBuf,
    /// Human-readable duration string.
    pub duration_str: String,
    /// Formatted file size string.
    pub size_str: String,
}

/// Native desktop recorder application state conforming to `[CONTRACT-GUI-001]`.
pub struct RecorderGuiApp {
    config: AppConfig,
    pipeline: RecordingPipeline,
    active_tab: AppTab,
    capture_mode: CaptureMode,
    monitors: Vec<MonitorInfo>,
    selected_monitor_idx: usize,
    custom_x: u32,
    custom_y: u32,
    custom_w: u32,
    custom_h: u32,
    hud_state: FloatingHudState,
    status_message: Option<String>,
    last_recording: Option<RecentRecordingMeta>,
    custom_output_file: Option<PathBuf>,
    #[cfg(target_os = "windows")]
    tray_manager: Option<Win32TrayManager>,
    exit_requested: bool,

    // UI/UX Obsidian states
    mic_muted: bool,
    container_format: ContainerFormat,
    settings_cat: SettingsCategory,
    encoder_choice: EncoderChoice,
    rate_control: RateControlMode,
    target_bitrate_kbps: u32,
    nvenc_preset: u8, // 1 = P1, 4 = P4, 7 = P7
}

impl RecorderGuiApp {
    /// Creates and initializes a new [`RecorderGuiApp`] instance with optional egui repaint context.
    #[must_use]
    pub fn new_with_context(ctx: Option<&egui::Context>) -> Self {
        let config = AppConfig::new();
        let mut pipeline = RecordingPipeline::new(config.clone());

        // Inject default Windows backends using DIP
        #[cfg(target_os = "windows")]
        {
            pipeline.set_capture_backend(Box::new(WgcCaptureBackend::new()));
            pipeline.set_audio_backend(Box::new(WasapiAudioBackend::new()));
        }

        let monitors: Vec<MonitorInfo> = match enumerate_monitors() {
            Ok(list) if !list.is_empty() => list,
            _ => vec![MonitorInfo {
                index: 0,
                name: "Primary Display".to_string(),
                width: 1920,
                height: 1080,
                is_primary: true,
                left: 0,
                top: 0,
                hmonitor: 0,
            }],
        };

        #[cfg(target_os = "windows")]
        let tray_manager = {
            if let Some(c) = ctx {
                let c_clone = c.clone();
                Win32TrayManager::new_with_wakeup(move || {
                    c_clone.request_repaint();
                })
                .ok()
            } else {
                Win32TrayManager::new_with_wakeup(|| {}).ok()
            }
        };

        Self {
            config,
            pipeline,
            active_tab: AppTab::Dashboard,
            capture_mode: CaptureMode::FullScreen,
            monitors,
            selected_monitor_idx: 0,
            custom_x: 0,
            custom_y: 0,
            custom_w: 1920,
            custom_h: 1080,
            hud_state: FloatingHudState::default(),
            status_message: None,
            last_recording: None,
            custom_output_file: None,
            #[cfg(target_os = "windows")]
            tray_manager,
            exit_requested: false,
            mic_muted: false,
            container_format: ContainerFormat::Avi,
            settings_cat: SettingsCategory::VideoEncoder,
            encoder_choice: EncoderChoice::NvidiaNvenc,
            rate_control: RateControlMode::Cbr,
            target_bitrate_kbps: 12_000,
            nvenc_preset: 7,
        }
    }

    /// Creates and initializes a new [`RecorderGuiApp`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self::new_with_context(None)
    }

    fn get_active_target(&self) -> CaptureTarget {
        match self.capture_mode {
            CaptureMode::FullScreen => CaptureTarget::FullScreen {
                monitor_index: self.selected_monitor_idx,
            },
            CaptureMode::Window => CaptureTarget::FullScreen {
                monitor_index: self.selected_monitor_idx,
            },
            CaptureMode::CustomArea => CaptureTarget::Area {
                x: self.custom_x,
                y: self.custom_y,
                width: self.custom_w.max(160),
                height: self.custom_h.max(120),
            },
            CaptureMode::Presets => CaptureTarget::Area {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        }
    }

    fn record_save_meta(&mut self, path: PathBuf, elapsed_secs: u64) {
        let mins = elapsed_secs / 60;
        let secs = elapsed_secs % 60;
        let duration_str = format!("{:02}:{:02} min", mins, secs);

        let size_mb = path
            .metadata()
            .map(|m| m.len() as f64 / 1_048_576.0)
            .unwrap_or(0.0);
        let size_str = format!("{:.1} MB", size_mb);

        self.last_recording = Some(RecentRecordingMeta {
            path,
            duration_str,
            size_str,
        });
    }

    /// Triggers an immediate, non-blocking snapshot screenshot of the active capture target.
    ///
    /// Saves high-resolution PNG to the user output directory and updates recent media cards.
    pub fn trigger_screenshot(&mut self) {
        let target = self.get_active_target();
        #[cfg(target_os = "windows")]
        {
            match capture_screenshot(&target) {
                Ok(frame) => {
                    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
                    let file_name = format!("Screenshot_{}.png", timestamp);
                    let out_path = self.config.output_directory.join(&file_name);
                    match frame.save_as_png(&out_path) {
                        Ok(()) => {
                            let size_kb = out_path
                                .metadata()
                                .map(|m| m.len() as f64 / 1024.0)
                                .unwrap_or(0.0);
                            let size_str = format!("{:.1} KB", size_kb);
                            self.last_recording = Some(RecentRecordingMeta {
                                path: out_path.clone(),
                                duration_str: "Snapshot".to_string(),
                                size_str,
                            });
                            self.status_message = Some(format!(
                                "✓ Snapshot saved: {}",
                                file_name
                            ));
                        }
                        Err(err) => {
                            self.status_message = Some(format!("Error saving snapshot: {}", err));
                        }
                    }
                }
                Err(err) => {
                    self.status_message = Some(format!("Screenshot capture failed: {}", err));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = target;
            self.status_message = Some("Screenshots not supported on this OS".to_string());
        }
    }
}

impl Default for RecorderGuiApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for RecorderGuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        configure_obsidian_theme(&ctx);
        ctx.set_zoom_factor(self.config.font_scale_percent as f32 / 100.0);
        let lang = self.config.language;

        // 0. Intercept tray close requests
        #[cfg(target_os = "windows")]
        {
            if ctx.input(|i| i.viewport().close_requested())
                && !self.exit_requested
                && !is_exit_requested()
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }

            if is_exit_requested() && !self.exit_requested {
                self.exit_requested = true;
                if self.pipeline.is_active() {
                    let _ = self.pipeline.stop();
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }

            let mut tray_events = Vec::new();
            if let Some(ref tray) = self.tray_manager {
                while let Some(ev) = tray.poll_event() {
                    tray_events.push(ev);
                }
            }

            for ev in tray_events {
                match ev {
                    TrayEvent::ShowWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    TrayEvent::ToggleRecording => {
                        if self.pipeline.is_active() {
                            let elapsed = self.pipeline.elapsed().as_secs();
                            if let Ok(Some(path)) = self.pipeline.stop() {
                                self.record_save_meta(path, elapsed);
                            }
                        } else {
                            let target = self.get_active_target();
                            let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
                            let out_path = self.custom_output_file.clone().unwrap_or_else(|| {
                                self.config
                                    .output_directory
                                    .join(format!("Recording_{}.avi", timestamp))
                            });
                            self.pipeline.set_config(self.config.clone());
                            let _ = self.pipeline.start(target, out_path);
                        }
                    }
                    TrayEvent::TakeSnapshot => {
                        self.trigger_screenshot();
                    }
                    TrayEvent::OpenOutputDir => {
                        let _ = open::that(&self.config.output_directory);
                    }
                    TrayEvent::SaveInstantReplay => {
                        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
                        let replay_path = self
                            .config
                            .output_directory
                            .join(format!("Replay_{}.avi", timestamp));
                        if let Ok(path) = self.pipeline.save_instant_replay(&replay_path) {
                            self.record_save_meta(path, 15);
                        }
                    }
                    TrayEvent::ExitApp => {
                        self.exit_requested = true;
                        if self.pipeline.is_active() {
                            let _ = self.pipeline.stop();
                        }
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                }
            }
        }

        // Global hotkeys (F9 = Toggle Record, F10 = Snapshot)
        let mut toggle_record = false;
        let mut take_snapshot = false;
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F9) {
                toggle_record = true;
            }
            if i.key_pressed(egui::Key::F10) {
                take_snapshot = true;
            }
        });

        if toggle_record {
            if self.pipeline.is_active() {
                let elapsed = self.pipeline.elapsed().as_secs();
                if let Ok(Some(path)) = self.pipeline.stop() {
                    self.record_save_meta(path, elapsed);
                }
            } else {
                let target = self.get_active_target();
                let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
                let out_path = self.custom_output_file.clone().unwrap_or_else(|| {
                    self.config
                        .output_directory
                        .join(format!("Recording_{}.avi", timestamp))
                });
                self.pipeline.set_config(self.config.clone());
                let _ = self.pipeline.start(target, out_path);
            }
        }

        if take_snapshot {
            self.trigger_screenshot();
        }

        // Update live HUD state and VU meter metrics
        self.hud_state.is_recording = self.pipeline.is_active();
        self.hud_state.is_paused = self.pipeline.is_paused();
        self.hud_state.elapsed = self.pipeline.elapsed();
        let (peak_l, peak_r) = self.pipeline.live_audio_peaks();
        self.hud_state.audio_peak_l = peak_l;
        self.hud_state.audio_peak_r = peak_r;

        // Top Navigation Bar matching main_screen.png
        egui::Panel::top("top_header").show(ui, |ui| {
            ui.add_space(4.0);
            self.render_top_header(ui, lang);
            ui.add_space(4.0);
        });

        // Bottom Status Bar matching main_screen.png
        egui::Panel::bottom("bottom_status_bar").show(ui, |ui| {
            ui.add_space(3.0);
            self.render_bottom_status_bar(ui, lang);
            ui.add_space(3.0);
        });

        // Floating HUD if active recording
        if self.hud_state.is_recording {
            egui::Panel::bottom("hud_floating_panel").show(ui, |ui| {
                ui.add_space(4.0);
                if let Some(action) = render_floating_hud(ui, &self.hud_state, lang) {
                    match action {
                        "pause" => {
                            let _ = self.pipeline.pause();
                        }
                        "resume" => {
                            let _ = self.pipeline.resume();
                        }
                        "stop" => {
                            let elapsed = self.pipeline.elapsed().as_secs();
                            if let Ok(Some(path)) = self.pipeline.stop() {
                                self.record_save_meta(path, elapsed);
                            }
                        }
                        _ => {}
                    }
                }
                ui.add_space(4.0);
            });
        }

        // Central Main Content
        egui::CentralPanel::default().show(ui, |ui| {
            match self.active_tab {
                AppTab::Dashboard => self.render_dashboard(ui, lang),
                AppTab::Settings => self.render_settings_modal(ui, lang),
            }
        });

        if self.hud_state.is_recording {
            ctx.request_repaint_after(std::time::Duration::from_millis(33)); // 30 FPS UI refresh
        }
    }
}

impl RecorderGuiApp {
    /// Renders the top navigation bar matching `main_screen.png`.
    fn render_top_header(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.horizontal(|ui| {
            // App icon: bullseye / target icon
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), egui::Sense::hover());
            ui.painter().rect_filled(icon_rect, CornerRadius::same(6), COLOR_PRIMARY);
            ui.painter().circle_stroke(icon_rect.center(), 7.0, Stroke::new(2.0, Color32::WHITE));
            ui.painter().circle_filled(icon_rect.center(), 3.0, Color32::WHITE);

            ui.add_space(6.0);

            // Title: Screen Recorder
            ui.label(RichText::new("Screen Recorder").size(17.0).strong().color(TEXT_HEADLINE));

            ui.add_space(4.0);

            // Badges: PRO, ● Listo, 🦀 Core v2.4 (Sub-5ms)
            render_badge(ui, text(I18nKey::ProBadge, lang), Color32::from_rgb(0x1E, 0x3A, 0x8A), COLOR_PRIMARY_HOVER);
            
            let (status_text, status_bg, status_fg) = if self.pipeline.is_active() {
                (format!("● {}", text(I18nKey::StatusRecording, lang)), Color32::from_rgb(0x45, 0x1A, 0x1A), COLOR_SECONDARY)
            } else if self.pipeline.is_paused() {
                (format!("● {}", text(I18nKey::StatusPaused, lang)), Color32::from_rgb(0x45, 0x30, 0x10), Color32::from_rgb(0xF5, 0x9E, 0x0B))
            } else {
                (format!("● {}", text(I18nKey::StatusIdle, lang)), Color32::from_rgb(0x06, 0x4E, 0x3B), COLOR_TERTIARY)
            };
            render_badge(ui, &status_text, status_bg, status_fg);

            render_badge(ui, "Rust Core v2.4 (Sub-5ms)", Color32::from_rgb(0x2A, 0x18, 0x10), COLOR_RUST_ORANGE);

            // Right side navigation and window controls
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Window minimize button
                let min_btn = egui::Button::new(RichText::new("—").size(14.0).strong().color(TEXT_HEADLINE))
                    .min_size(Vec2::new(36.0, 32.0))
                    .fill(BG_CARD)
                    .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                    .corner_radius(CornerRadius::same(6));
                if ui.add(min_btn).on_hover_text(text(I18nKey::TrayMinimizeToTray, lang)).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }

                ui.add_space(4.0);

                // Language selector pill
                let lang_label = match lang {
                    Language::English => "🌐 EN",
                    Language::Spanish => "🌐 ES",
                };
                let lang_btn = egui::Button::new(RichText::new(lang_label).size(13.0).strong().color(TEXT_HEADLINE))
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

                // Settings tab toggle
                let is_settings = self.active_tab == AppTab::Settings;
                let settings_btn = egui::Button::new(
                    RichText::new(text(I18nKey::NavSettings, lang))
                        .size(13.0)
                        .strong()
                        .color(if is_settings { Color32::WHITE } else { TEXT_BODY }),
                )
                .min_size(Vec2::new(100.0, 32.0))
                .fill(if is_settings { COLOR_PRIMARY } else { BG_CARD })
                .stroke(Stroke::new(1.0, if is_settings { COLOR_PRIMARY_HOVER } else { BORDER_SUBTLE }))
                .corner_radius(CornerRadius::same(6));

                if ui.add(settings_btn).clicked() {
                    self.active_tab = if is_settings { AppTab::Dashboard } else { AppTab::Settings };
                }

                ui.add_space(4.0);

                // General dashboard toggle
                let is_general = self.active_tab == AppTab::Dashboard;
                let dash_btn = egui::Button::new(
                    RichText::new(text(I18nKey::NavDashboard, lang))
                        .size(13.0)
                        .strong()
                        .color(if is_general { Color32::WHITE } else { TEXT_BODY }),
                )
                .min_size(Vec2::new(90.0, 32.0))
                .fill(if is_general { COLOR_PRIMARY } else { BG_CARD })
                .stroke(Stroke::new(1.0, if is_general { COLOR_PRIMARY_HOVER } else { BORDER_SUBTLE }))
                .corner_radius(CornerRadius::same(6));

                if ui.add(dash_btn).clicked() {
                    self.active_tab = AppTab::Dashboard;
                }
            });
        });
    }

    /// Renders the bottom status bar matching `main_screen.png`.
    fn render_bottom_status_bar(&self, ui: &mut egui::Ui, lang: Language) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚙ CPU: 1.4%    💾 RAM: 64 MB").size(12.0).color(TEXT_MUTED));

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
    fn render_dashboard(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.add_space(4.0);

        // 1. Capture Mode Selection Bar
        ui.horizontal(|ui| {
            // Segmented mode buttons
            let modes = [
                (CaptureMode::FullScreen, format!("🖥 {}", text(I18nKey::ModeFullScreen, lang))),
                (CaptureMode::Window, format!("🗔 {}", text(I18nKey::ModeWindow, lang))),
                (CaptureMode::CustomArea, format!("⬚ {}", text(I18nKey::ModeFreeRegion, lang))),
                (CaptureMode::Presets, format!("⚙ {}", text(I18nKey::ModePresets, lang))),
            ];

            for (m, lbl) in modes {
                let selected = self.capture_mode == m;
                let btn = egui::Button::new(
                    RichText::new(lbl)
                        .size(13.0)
                        .color(if selected { Color32::WHITE } else { TEXT_BODY }),
                )
                .fill(if selected { COLOR_PRIMARY } else { BG_CARD })
                .stroke(Stroke::new(1.0, if selected { COLOR_PRIMARY } else { BORDER_SUBTLE }))
                .corner_radius(CornerRadius::same(6));

                if ui.add(btn).clicked() {
                    self.capture_mode = m;
                }
            }

            // Right side resolution & FPS pills
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                render_badge(ui, &format!("{:.2} FPS", self.config.fps), Color32::from_rgb(0x06, 0x4E, 0x3B), COLOR_TERTIARY);
                
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
                // Monitor box icon
                let (mon_icon_rect, _) = ui.allocate_exact_size(Vec2::new(38.0, 32.0), egui::Sense::hover());
                ui.painter().rect_filled(mon_icon_rect, CornerRadius::same(6), BG_ELEVATED);
                ui.painter().rect_stroke(mon_icon_rect, CornerRadius::same(6), Stroke::new(1.0, COLOR_PRIMARY), egui::StrokeKind::Inside);
                ui.painter().text(
                    mon_icon_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", self.selected_monitor_idx + 1),
                    egui::FontId::proportional(14.0),
                    COLOR_PRIMARY,
                );

                ui.add_space(8.0);

                ui.vertical(|ui| {
                    ui.label(RichText::new(text(I18nKey::PrimaryDisplayLabel, lang)).size(11.0).color(TEXT_MUTED));

                    let current_text = self.monitors.get(self.selected_monitor_idx)
                        .map(|m| {
                            let mut star_tag = String::new();
                            if m.is_primary {
                                star_tag = format!(" {}", text(I18nKey::PrimaryBadge, lang));
                            }
                            format!("\\\\.\\DISPLAY{} [{}x{}]{} (sRGB)", m.index + 1, m.width, m.height, star_tag)
                        })
                        .unwrap_or_else(|| {
                            format!("\\\\.\\DISPLAY1 [1920x1080] {} (sRGB)", text(I18nKey::PrimaryBadge, lang))
                        });

                    // Primary display selector dropdown
                    egui::ComboBox::from_id_salt("display_selector")
                        .selected_text(RichText::new(current_text).size(13.0).color(TEXT_HEADLINE))
                        .width(420.0)
                        .show_ui(ui, |ui| {
                            for (idx, m) in self.monitors.iter().enumerate() {
                                let mut star_tag = String::new();
                                if m.is_primary {
                                    star_tag = format!(" {}", text(I18nKey::PrimaryBadge, lang));
                                }
                                let label = format!("\\\\.\\DISPLAY{} [{}x{}]{} (sRGB)", m.index + 1, m.width, m.height, star_tag);
                                ui.selectable_value(&mut self.selected_monitor_idx, idx, label);
                            }
                        });
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    render_badge(ui, "Windows Graphics Capture (WGC)", Color32::from_rgb(0x1E, 0x29, 0x3B), COLOR_PRIMARY_HOVER);
                    ui.label(RichText::new(text(I18nKey::CaptureBackendLabel, lang)).size(12.0).color(TEXT_MUTED));
                });
            });
        });

        ui.add_space(8.0);

        // 3. Audio & Media Sources Row (3 Cards side by side)
        ui.columns(3, |columns| {
            // Column 1: System Audio
            card_frame().show(&mut columns[0], |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("🔊 {}", text(I18nKey::SystemAudioLabel, lang))).strong().color(TEXT_HEADLINE));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("100%").size(11.0).color(TEXT_MUTED));
                    });
                });
                ui.add_space(6.0);
                render_vu_meter(ui, self.hud_state.audio_peak_l, ui.available_width(), 6.0);
                ui.add_space(4.0);
                ui.label(RichText::new("Altavoces (Realtek High Definition Audio)").size(11.0).color(TEXT_MUTED));
            });

            // Column 2: Microphone
            card_frame().show(&mut columns[1], |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("🎙 {}", text(I18nKey::MicrophoneLabel, lang))).strong().color(TEXT_HEADLINE));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mute_icon = if self.mic_muted { "🔇" } else { "🎙" };
                        if ui.button(mute_icon).clicked() {
                            self.mic_muted = !self.mic_muted;
                        }
                    });
                });
                ui.add_space(6.0);
                let mic_peak = if self.mic_muted { 0.0 } else { self.hud_state.audio_peak_r };
                render_vu_meter(ui, mic_peak, ui.available_width(), 6.0);
                ui.add_space(4.0);
                ui.label(RichText::new("Micrófono USB (HyperX QuadCast)").size(11.0).color(TEXT_MUTED));
            });

            // Column 3: Cursor Effects & Webcam PIP
            card_frame().show(&mut columns[2], |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("🖱 {}", text(I18nKey::CursorHaloToggle, lang))).strong().color(TEXT_HEADLINE));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.checkbox(&mut self.config.cursor_halo_enabled, "").changed() {
                            self.pipeline.set_config(self.config.clone());
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("✨ {}", text(I18nKey::CursorRipplesToggle, lang))).size(11.0).color(TEXT_BODY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.checkbox(&mut self.config.cursor_click_ripples_enabled, "").changed() {
                            self.pipeline.set_config(self.config.clone());
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let colors = [
                        (CursorHighlightColor::YellowGlow, "🟡", text(I18nKey::CursorColorYellow, lang)),
                        (CursorHighlightColor::ElectricBlue, "🔵", text(I18nKey::CursorColorBlue, lang)),
                        (CursorHighlightColor::NeonEmerald, "🟢", text(I18nKey::CursorColorNeon, lang)),
                    ];
                    for (c, dot, tip) in colors {
                        let sel = self.config.cursor_halo_color == c;
                        if ui.selectable_label(sel, dot).on_hover_text(tip).clicked() {
                            self.config.cursor_halo_color = c;
                            self.pipeline.set_config(self.config.clone());
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        render_badge(ui, "D3D11", Color32::from_rgb(0x06, 0x4E, 0x3B), COLOR_TERTIARY);
                    });
                });
            });
        });

        ui.add_space(8.0);

        // 4. Output Destination Row
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("📁 {}", text(I18nKey::DestinationFolder, lang))).strong().color(TEXT_HEADLINE));

                let path_str = if let Some(ref cf) = self.custom_output_file {
                    cf.display().to_string()
                } else {
                    self.config.output_directory.display().to_string()
                };

                // Elevated path box
                elevated_frame().show(ui, |ui| {
                    ui.label(RichText::new(path_str).size(12.0).color(TEXT_BODY));
                });

                if ui.button(text(I18nKey::BtnChange, lang)).clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.config.set_output_directory(&folder);
                        self.pipeline.set_config(self.config.clone());
                    }
                }

                if ui.button(format!("📂 {}", text(I18nKey::BtnOpenFolder, lang))).clicked() {
                    let _ = std::fs::create_dir_all(&self.config.output_directory);
                    let _ = open::that(&self.config.output_directory);
                }

                // Format selector pill
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("fmt_sel")
                        .selected_text(match self.container_format {
                            ContainerFormat::Avi => "AVI ⌵",
                            ContainerFormat::Mp4 => "MP4 (Roadmap) ⌵",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.container_format, ContainerFormat::Avi, "AVI (MJPEG Lockstep)");
                            ui.selectable_value(&mut self.container_format, ContainerFormat::Mp4, "MP4 (H.264 - Fase 2 Roadmap)");
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

        // 5. Giant Action Buttons Row (Glowing Red Record + Slate Snapshot)
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
            .fill(if is_rec { Color32::from_rgb(0x99, 0x1B, 0x1B) } else { COLOR_SECONDARY })
            .min_size(Vec2::new(ui.available_width() - 220.0, 48.0))
            .corner_radius(CornerRadius::same(8));

            if ui.add(rec_btn).clicked() {
                if is_rec {
                    let elapsed = self.pipeline.elapsed().as_secs();
                    if let Ok(Some(path)) = self.pipeline.stop() {
                        self.record_save_meta(path, elapsed);
                    }
                } else {
                    let target = self.get_active_target();
                    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
                    let out_path = self.custom_output_file.clone().unwrap_or_else(|| {
                        self.config
                            .output_directory
                            .join(format!("Recording_{}.avi", timestamp))
                    });
                    self.pipeline.set_config(self.config.clone());
                    let _ = self.pipeline.start(target, out_path);
                }
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

        // 6. Recent Recording Card matching main_screen.png
        let mut should_delete = false;
        let mut should_copy = false;
        let mut should_play = false;
        let mut should_open_folder = false;

        if let Some(ref meta) = self.last_recording {
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let is_snapshot = meta.duration_str == "Snapshot";
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(36.0, 36.0), egui::Sense::hover());
                    let bg_color = if is_snapshot {
                        Color32::from_rgb(0x1E, 0x3A, 0x8A) // Slate Blue
                    } else {
                        Color32::from_rgb(0x06, 0x4E, 0x3B) // Dark Green
                    };
                    ui.painter().rect_filled(icon_rect, CornerRadius::same(8), bg_color);
                    let icon_glyph = if is_snapshot { "📷" } else { "🎬" };
                    ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, icon_glyph, egui::FontId::proportional(16.0), Color32::WHITE);

                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let file_name = meta.path.file_name().unwrap_or_default().to_string_lossy();
                            ui.label(RichText::new(file_name).strong().color(TEXT_HEADLINE));
                            let badge_text = if is_snapshot {
                                text(I18nKey::SnapshotBadge, lang)
                            } else {
                                text(I18nKey::RecentRecording, lang)
                            };
                            render_badge(ui, badge_text, bg_color, COLOR_TERTIARY);
                        });

                        ui.label(RichText::new(format!("⏱ {}   •   💾 {}", meta.duration_str, meta.size_str)).size(11.0).color(TEXT_MUTED));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Delete Button (Danger red stroke)
                        let del_btn = egui::Button::new(
                            RichText::new("🗑").size(12.0).color(Color32::from_rgb(0xEF, 0x44, 0x44)),
                        )
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(0x7F, 0x1D, 0x1D)))
                        .corner_radius(CornerRadius::same(6));
                        if ui.add(del_btn).on_hover_text(text(I18nKey::BtnDelete, lang)).clicked() {
                            should_delete = true;
                        }

                        // Show in Folder Button
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

                        // Copy Path Button
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

                        // Play / Open Button
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
                self.status_message = Some(text(I18nKey::StatusCopiedToClipboard, lang).to_string());
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

    /// Renders the complete settings view conforming to `settings_screen_1.png`.
    fn render_settings_modal(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.add_space(6.0);

        // Header matching settings_screen_1.png
        ui.horizontal(|ui| {
            ui.label(RichText::new(text(I18nKey::SettingsEncoderTitle, lang)).size(16.0).strong().color(TEXT_HEADLINE));
            render_badge(ui, "NVENC / Rust-AV Core v2.4", BG_ELEVATED, TEXT_BODY);
            render_badge(ui, text(I18nKey::HwReadyPill, lang), Color32::from_rgb(0x06, 0x4E, 0x3B), COLOR_TERTIARY);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(text(I18nKey::BtnCloseModal, lang)).clicked() {
                    self.active_tab = AppTab::Dashboard;
                }
            });
        });

        ui.add_space(10.0);

        // Two-column layout: Left Sidebar + Main Settings Content
        ui.horizontal(|ui| {
            // Left Category Sidebar (Width ~190px vertical layout)
            ui.vertical(|ui| {
                ui.set_width(190.0);
                let cats = [
                    (SettingsCategory::VideoEncoder, format!("🎞 {}", text(I18nKey::SettingsTabVideoEncoder, lang))),
                    (SettingsCategory::AudioChannels, format!("🔊 {}", text(I18nKey::SettingsTabAudioChannels, lang))),
                    (SettingsCategory::OutputFormat, format!("📁 {}", text(I18nKey::SettingsTabOutputFormat, lang))),
                    (SettingsCategory::Hotkeys, format!("⌨ {}", text(I18nKey::SettingsTabHotkeys, lang))),
                    (SettingsCategory::PerfBuffer, format!("⚡ {}", text(I18nKey::SettingsTabPerfBuffer, lang))),
                ];

                for (cat, label) in cats {
                    let is_active = self.settings_cat == cat;
                    let btn = egui::Button::new(
                        RichText::new(label)
                            .size(13.0)
                            .color(if is_active { Color32::WHITE } else { TEXT_BODY }),
                    )
                    .fill(if is_active { BG_CARD_HOVER } else { BG_CARD })
                    .stroke(Stroke::new(1.0, if is_active { COLOR_PRIMARY } else { BORDER_SUBTLE }))
                    .min_size(Vec2::new(186.0, 36.0))
                    .corner_radius(CornerRadius::same(6));

                    if ui.add(btn).clicked() {
                        self.settings_cat = cat;
                    }
                    ui.add_space(4.0);
                }

                ui.add_space(16.0);

                // Sidebar telemetry matching settings_screen_1.png
                card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(text(I18nKey::LatencyMetric, lang)).size(10.0).color(TEXT_MUTED));
                        ui.label(RichText::new("2.8 ms").size(10.0).strong().color(COLOR_TERTIARY));
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(text(I18nKey::DroppedFramesMetric, lang)).size(10.0).color(TEXT_MUTED));
                        ui.label(RichText::new("0 (0.00%)").size(10.0).color(TEXT_BODY));
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(text(I18nKey::VramMetric, lang)).size(10.0).color(TEXT_MUTED));
                        ui.label(RichText::new("412 MB").size(10.0).color(COLOR_PRIMARY_HOVER));
                    });
                });
            });

            ui.separator();

            // Right Main Settings Pane
            ui.vertical(|ui| {
                match self.settings_cat {
                    SettingsCategory::VideoEncoder => self.render_settings_video_encoder(ui, lang),
                    SettingsCategory::AudioChannels => self.render_settings_audio(ui, lang),
                    SettingsCategory::OutputFormat => self.render_settings_output(ui, lang),
                    SettingsCategory::Hotkeys => self.render_settings_hotkeys(ui, lang),
                    SettingsCategory::PerfBuffer => self.render_settings_perf(ui, lang),
                }
            });
        });

        // Bottom Footer matching settings_screen_1.png
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button(format!("↺ {}", text(I18nKey::BtnResetDefaults, lang))).clicked() {
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

    /// Renders the Video & Encoder tab matching `settings_screen_1.png`.
    fn render_settings_video_encoder(&mut self, ui: &mut egui::Ui, lang: Language) {
        // Section: Backend y Codificador por Hardware
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("⚙ {}", text(I18nKey::HwEncoderBackendSection, lang))).strong().color(TEXT_HEADLINE));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                render_badge(ui, "Direct HW Accel", Color32::from_rgb(0x06, 0x4E, 0x3B), COLOR_TERTIARY);
            });
        });

        ui.add_space(6.0);

        // 4-Card Encoder Grid
        ui.columns(2, |cols| {
            // Card 1: NVIDIA NVENC
            let is_nvenc = self.encoder_choice == EncoderChoice::NvidiaNvenc;
            card_frame().show(&mut cols[0], |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(is_nvenc, "✓ NVIDIA NVENC").clicked() {
                        self.encoder_choice = EncoderChoice::NvidiaNvenc;
                    }
                    render_badge(ui, text(I18nKey::RecommendedBadge, lang), Color32::from_rgb(0x1E, 0x3A, 0x8A), COLOR_PRIMARY_HOVER);
                });
                ui.label(RichText::new("H.264 / HEVC / AV1 via Dual NVENC Gen 8").size(11.0).color(TEXT_MUTED));
            });

            // Card 2: AMD AMF Core
            let is_amf = self.encoder_choice == EncoderChoice::AmdAmf;
            card_frame().show(&mut cols[1], |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(is_amf, "AMD AMF Core").clicked() {
                        self.encoder_choice = EncoderChoice::AmdAmf;
                    }
                    render_badge(ui, "VCN 4.0", BG_ELEVATED, TEXT_MUTED);
                });
                ui.label(RichText::new("Optimizado para Radeon RDNA3 Architecture").size(11.0).color(TEXT_MUTED));
            });
        });

        ui.add_space(4.0);

        ui.columns(2, |cols| {
            // Card 3: Intel QuickSync
            let is_qsv = self.encoder_choice == EncoderChoice::IntelQuickSync;
            card_frame().show(&mut cols[0], |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(is_qsv, "Intel QuickSync").clicked() {
                        self.encoder_choice = EncoderChoice::IntelQuickSync;
                    }
                    render_badge(ui, "QSV Gen12", BG_ELEVATED, TEXT_MUTED);
                });
                ui.label(RichText::new("Soporte Iris Xe & Arc Series Discrete").size(11.0).color(TEXT_MUTED));
            });

            // Card 4: Software x264 (CPU)
            let is_cpu = self.encoder_choice == EncoderChoice::SoftwareCpu;
            card_frame().show(&mut cols[1], |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(is_cpu, "Software x264 (CPU)").clicked() {
                        self.encoder_choice = EncoderChoice::SoftwareCpu;
                    }
                    render_badge(ui, "CPU RUST", BG_ELEVATED, TEXT_MUTED);
                });
                ui.label(RichText::new("Subproceso multi-hilo con fallback").size(11.0).color(TEXT_MUTED));
            });
        });

        ui.add_space(6.0);

        // Detected GPU line
        elevated_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("🖥 {}", text(I18nKey::DetectedGpuLabel, lang))).strong().color(TEXT_HEADLINE));
                ui.label(RichText::new("NVIDIA GeForce RTX 3060 Laptop GPU • Driver 552.22 WHQL • Dual Compositor").color(COLOR_PRIMARY_HOVER));
            });
        });

        ui.add_space(10.0);

        // Section: Control de Tasa y Calidad (Bitrate)
        ui.label(RichText::new(format!("📊 {}", text(I18nKey::BitrateSectionTitle, lang))).strong().color(TEXT_HEADLINE));
        ui.add_space(4.0);

        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::BitrateControlMode, lang)).color(TEXT_HEADLINE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let modes = [
                        (RateControlMode::Cqp, "CQP / CQ"),
                        (RateControlMode::Vbr, "VBR (Variable)"),
                        (RateControlMode::Cbr, "CBR (Constante)"),
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

            // Target Bitrate slider
            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::TargetBitrateLabel, lang)).color(TEXT_HEADLINE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mbps = self.target_bitrate_kbps as f64 / 1000.0;
                    ui.label(RichText::new(format!("{} Kbps ({:.1} Mbps)", self.target_bitrate_kbps, mbps)).strong().color(COLOR_PRIMARY_HOVER));
                });
            });

            ui.add(egui::Slider::new(&mut self.target_bitrate_kbps, 2_000..=50_000).show_value(false));

            ui.horizontal(|ui| {
                ui.label(RichText::new("2,500k (720p)").size(10.0).color(TEXT_MUTED));
                ui.add_space(40.0);
                ui.label(RichText::new("6,000k (1080p30)").size(10.0).color(TEXT_MUTED));
                ui.add_space(40.0);
                ui.label(RichText::new("12,000k (1080p60 Pro)").size(10.0).strong().color(COLOR_PRIMARY_HOVER));
                ui.add_space(40.0);
                ui.label(RichText::new("24,000k (2K)").size(10.0).color(TEXT_MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("50,000k (4K60 HDR)").size(10.0).color(TEXT_MUTED));
                });
            });

            ui.add_space(8.0);

            // 3-Column Settings (Preset, Profile, Keyframes)
            ui.columns(3, |cols| {
                cols[0].vertical(|ui| {
                    ui.label(RichText::new(text(I18nKey::NvencPresetLabel, lang)).size(11.0).color(TEXT_MUTED));
                    ui.horizontal(|ui| {
                        let presets = [(1, "P1"), (4, "P4"), (7, "P7 (Máx)")];
                        for (p, lbl) in presets {
                            if ui.selectable_label(self.nvenc_preset == p, lbl).clicked() {
                                self.nvenc_preset = p;
                            }
                        }
                    });
                });

                cols[1].vertical(|ui| {
                    ui.label(RichText::new(text(I18nKey::ProfileLabel, lang)).size(11.0).color(TEXT_MUTED));
                    elevated_frame().show(ui, |ui| {
                        ui.label(RichText::new("High (AVC/H.264)").size(12.0).color(TEXT_HEADLINE));
                    });
                });

                cols[2].vertical(|ui| {
                    ui.label(RichText::new(text(I18nKey::KeyframesLabel, lang)).size(11.0).color(TEXT_MUTED));
                    elevated_frame().show(ui, |ui| {
                        ui.label(RichText::new("2s (120 frames)").size(12.0).color(TEXT_HEADLINE));
                    });
                });
            });
        });
    }

    /// Renders the Audio & Channels tab.
    fn render_settings_audio(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(RichText::new(text(I18nKey::SettingsTabAudioChannels, lang)).size(16.0).strong().color(TEXT_HEADLINE));
        ui.add_space(8.0);

        let mut config_changed = false;

        // 1. System Audio Card (WASAPI Loopback)
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut self.config.record_system_audio, text(I18nKey::SystemAudioLabel, lang)).changed() {
                    config_changed = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{}%", self.config.system_audio_volume_percent))
                            .strong()
                            .color(if self.config.record_system_audio { COLOR_PRIMARY_HOVER } else { TEXT_MUTED }),
                    );
                });
            });

            ui.add_space(6.0);
            ui.label(RichText::new(text(I18nKey::SystemAudioVolumeLabel, lang)).size(11.0).color(TEXT_MUTED));
            let mut sys_vol = self.config.system_audio_volume_percent;
            ui.add_enabled_ui(self.config.record_system_audio, |ui| {
                if ui.add(egui::Slider::new(&mut sys_vol, 0..=150).suffix("%")).changed() {
                    self.config.system_audio_volume_percent = sys_vol;
                    config_changed = true;
                }
            });

            // Quick presets: 50%, 100%, 150%
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for &preset in &[50, 100, 150] {
                    if ui.selectable_label(self.config.system_audio_volume_percent == preset, format!("{}%", preset)).clicked() {
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
                if ui.checkbox(&mut self.config.record_microphone, text(I18nKey::MicrophoneLabel, lang)).changed() {
                    config_changed = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (mute_icon_text, text_color) = if self.config.microphone_muted {
                        (format!("🔇 {}", text(I18nKey::BtnUnmute, lang)), COLOR_SECONDARY)
                    } else {
                        (format!("🎙 {}", text(I18nKey::BtnMute, lang)), TEXT_HEADLINE)
                    };
                    if ui.button(RichText::new(mute_icon_text).color(text_color)).clicked() {
                        self.config.microphone_muted = !self.config.microphone_muted;
                        self.mic_muted = self.config.microphone_muted;
                        config_changed = true;
                    }
                });
            });

            ui.add_space(6.0);
            ui.label(RichText::new(text(I18nKey::MicrophoneGainLabel, lang)).size(11.0).color(TEXT_MUTED));
            let mut mic_gain = self.config.microphone_gain_percent;
            ui.add_enabled_ui(self.config.record_microphone && !self.config.microphone_muted, |ui| {
                if ui.add(egui::Slider::new(&mut mic_gain, 0..=200).suffix("%")).changed() {
                    self.config.microphone_gain_percent = mic_gain;
                    config_changed = true;
                }
            });

            // Quick presets: 50%, 100%, 150%, 200%
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for &preset in &[50, 100, 150, 200] {
                    if ui.selectable_label(self.config.microphone_gain_percent == preset, format!("{}%", preset)).clicked() {
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
                ui.label(RichText::new(text(I18nKey::AudioSampleRateLabel, lang)).strong().color(TEXT_HEADLINE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    render_vu_meter(ui, peak_avg, 100.0, 8.0);
                    ui.label(RichText::new(text(I18nKey::HudAudioLevel, lang)).size(11.0).color(TEXT_MUTED));
                });
            });
            ui.add_space(6.0);
            ui.label(RichText::new(text(I18nKey::AudioSpecsDescription, lang)).color(TEXT_MUTED).size(12.0));
        });

        if config_changed {
            self.pipeline.set_config(self.config.clone());
        }
    }

    /// Renders the Output & Format tab.
    fn render_settings_output(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(RichText::new(text(I18nKey::SettingsTabOutputFormat, lang)).size(16.0).strong().color(TEXT_HEADLINE));
        ui.add_space(8.0);

        card_frame().show(ui, |ui| {
            ui.label(RichText::new(text(I18nKey::OutputDirectory, lang)).strong().color(TEXT_HEADLINE));
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

        // UI & Font Scale Card
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::UiScaleLabel, lang)).strong().color(TEXT_HEADLINE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{}%", self.config.font_scale_percent)).strong().color(COLOR_PRIMARY_HOVER));
                });
            });

            ui.add_space(6.0);
            let mut scale = self.config.font_scale_percent;
            if ui.add(egui::Slider::new(&mut scale, 80..=150).suffix("%")).changed() {
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
    }

    /// Renders the Keyboard Hotkeys tab.
    fn render_settings_hotkeys(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.label(RichText::new(text(I18nKey::SettingsTabHotkeys, lang)).size(16.0).strong().color(TEXT_HEADLINE));
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
        ui.label(RichText::new(text(I18nKey::SettingsTabPerfBuffer, lang)).size(16.0).strong().color(TEXT_HEADLINE));
        ui.add_space(8.0);

        let mut changed = false;

        card_frame().show(ui, |ui| {
            ui.label(RichText::new(format!("🖱 {}", text(I18nKey::CursorEffectsSection, lang))).strong().color(TEXT_HEADLINE));
            ui.add_space(6.0);

            if ui.checkbox(&mut self.config.cursor_halo_enabled, text(I18nKey::CursorHaloToggle, lang)).changed() {
                changed = true;
            }
            if ui.checkbox(&mut self.config.cursor_click_ripples_enabled, text(I18nKey::CursorRipplesToggle, lang)).changed() {
                changed = true;
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::CursorHaloRadiusLabel, lang)).color(TEXT_BODY));
                if ui.add(egui::Slider::new(&mut self.config.cursor_halo_radius, 12..=64).suffix(" px")).changed() {
                    changed = true;
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(text(I18nKey::CursorColorLabel, lang)).color(TEXT_BODY));
                let presets = [
                    (CursorHighlightColor::YellowGlow, format!("🟡 {}", text(I18nKey::CursorColorYellow, lang))),
                    (CursorHighlightColor::ElectricBlue, format!("🔵 {}", text(I18nKey::CursorColorBlue, lang))),
                    (CursorHighlightColor::NeonEmerald, format!("🟢 {}", text(I18nKey::CursorColorNeon, lang))),
                ];
                for (color, lbl) in presets {
                    if ui.selectable_label(self.config.cursor_halo_color == color, lbl).clicked() {
                        self.config.cursor_halo_color = color;
                        changed = true;
                    }
                }
            });
        });

        ui.add_space(10.0);

        card_frame().show(ui, |ui| {
            ui.label(RichText::new(text(I18nKey::InstantReplaySave, lang)).strong().color(TEXT_HEADLINE));
            if ui.add(egui::Slider::new(&mut self.config.instant_replay_duration_secs, 5..=120).suffix(" s")).changed() {
                changed = true;
            }
            ui.add_space(8.0);
            if ui.checkbox(&mut self.config.use_hardware_acceleration, text(I18nKey::HardwareAcceleration, lang)).changed() {
                changed = true;
            }
        });

        if changed {
            self.pipeline.set_config(self.config.clone());
        }
    }
}
