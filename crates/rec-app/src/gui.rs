//! Modern Obsidian Dark native desktop user interface built with `eframe`/`egui`.
//!
//! Conforms to `[SPEC-GUI-001]`, `[SPEC-UIUX-001]`, **SOLID (SRP, DIP)**, **DRY**, and **KISS**.
//!
//! # Module Architecture (SRP)
//! - [`models`]: Strongly-typed UI state enums and media metadata structs.
//! - [`dashboard`]: Top header, bottom status bar, and main recording dashboard view.
//! - [`settings`]: Categorized encoder, audio, output, hotkey, and performance settings modal.

pub mod dashboard;
pub mod models;
pub mod settings;

use std::path::PathBuf;
use eframe::egui;
use rec_core::config::AppConfig;
use rec_engine::pipeline::RecordingPipeline;
use rec_engine::traits::CaptureTarget;
use rec_platform_windows::{
    capture_screenshot, enumerate_monitors, MonitorInfo, WasapiAudioBackend, WgcCaptureBackend,
};
#[cfg(target_os = "windows")]
use rec_platform_windows::{is_exit_requested, TrayEvent, Win32TrayManager};

use crate::hud::{render_floating_hud, FloatingHudState, HudAction};
use crate::theme::configure_obsidian_theme;

pub use self::models::{
    AppTab, CaptureMode, ContainerFormat, EncoderChoice, RateControlMode, RecentMediaKind,
    RecentRecordingMeta, SettingsCategory,
};

/// Native desktop recorder application controller conforming to `[CONTRACT-GUI-001]`.
///
/// Orchestrates user interactions, system tray events, global hotkeys, and the underlying
/// [`RecordingPipeline`] while delegating layout rendering to [`dashboard`] and [`settings`].
pub struct RecorderGuiApp {
    pub(crate) config: AppConfig,
    pub(crate) pipeline: RecordingPipeline,
    pub(crate) active_tab: AppTab,
    pub(crate) capture_mode: CaptureMode,
    pub(crate) monitors: Vec<MonitorInfo>,
    pub(crate) selected_monitor_idx: usize,
    pub(crate) custom_x: u32,
    pub(crate) custom_y: u32,
    pub(crate) custom_w: u32,
    pub(crate) custom_h: u32,
    pub(crate) hud_state: FloatingHudState,
    pub(crate) status_message: Option<String>,
    pub(crate) last_recording: Option<RecentRecordingMeta>,
    pub(crate) custom_output_file: Option<PathBuf>,
    #[cfg(target_os = "windows")]
    pub(crate) tray_manager: Option<Win32TrayManager>,
    pub(crate) exit_requested: bool,
    pub(crate) container_format: ContainerFormat,
    pub(crate) settings_cat: SettingsCategory,
    pub(crate) encoder_choice: EncoderChoice,
    pub(crate) rate_control: RateControlMode,
    pub(crate) target_bitrate_kbps: u32,
    pub(crate) nvenc_preset: u8,
}

impl RecorderGuiApp {
    /// Creates and initializes a new [`RecorderGuiApp`] instance with an optional `egui::Context`
    /// for asynchronous system tray repaint wakeups.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Optional reference to the active `egui::Context`.
    ///
    /// # Returns
    ///
    /// A fully initialized [`RecorderGuiApp`] ready for rendering.
    #[must_use]
    pub fn new_with_context(ctx: Option<&egui::Context>) -> Self {
        let config = AppConfig::new();
        let mut pipeline = RecordingPipeline::new(config.clone());

        // Inject default Windows backends using Dependency Inversion (DIP)
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
            container_format: ContainerFormat::Avi,
            settings_cat: SettingsCategory::VideoEncoder,
            encoder_choice: EncoderChoice::NvidiaNvenc,
            rate_control: RateControlMode::Cbr,
            target_bitrate_kbps: 12_000,
            nvenc_preset: 7,
        }
    }

    /// Creates and initializes a new [`RecorderGuiApp`] instance without an initial `egui::Context`.
    ///
    /// # Returns
    ///
    /// A newly constructed [`RecorderGuiApp`] with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::new_with_context(None)
    }

    /// Resolves the active [`CaptureTarget`] from the current UI selection mode.
    #[must_use]
    pub(crate) fn get_active_target(&self) -> CaptureTarget {
        match self.capture_mode {
            CaptureMode::FullScreen | CaptureMode::Window => CaptureTarget::FullScreen {
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

    /// Starts a new recording session if the pipeline is currently idle (DRY controller action).
    pub fn start_recording(&mut self) {
        if self.pipeline.is_active() {
            return;
        }
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

    /// Stops the active recording session and updates [`RecentRecordingMeta`] (DRY controller action).
    pub fn stop_recording(&mut self) {
        if !self.pipeline.is_active() {
            return;
        }
        let elapsed = self.pipeline.elapsed().as_secs();
        if let Ok(Some(path)) = self.pipeline.stop() {
            self.record_save_meta(path, elapsed);
        }
    }

    /// Toggles between starting and stopping a recording session (DRY single entry point
    /// invoked by the Dashboard Record button, `F9` hotkey, and System Tray menu).
    pub fn toggle_recording(&mut self) {
        if self.pipeline.is_active() {
            self.stop_recording();
        } else {
            self.start_recording();
        }
    }

    /// Saves the circular Instant Replay buffer to disk and updates [`RecentRecordingMeta`].
    pub fn save_instant_replay(&mut self) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let replay_path = self
            .config
            .output_directory
            .join(format!("Replay_{}.avi", timestamp));
        let duration_secs = u64::from(self.config.instant_replay_duration_secs);
        if let Ok(path) = self.pipeline.save_instant_replay(&replay_path) {
            self.record_save_meta(path, duration_secs);
        }
    }

    /// Records file size and duration metadata for a finalized video file.
    pub(crate) fn record_save_meta(&mut self, path: PathBuf, elapsed_secs: u64) {
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
            kind: RecentMediaKind::Video { duration_str },
            size_str,
        });
    }

    /// Triggers an immediate, non-blocking snapshot screenshot of the active capture target.
    ///
    /// Saves a high-resolution PNG to the user output directory and updates recent media cards.
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
                                path: out_path,
                                kind: RecentMediaKind::Snapshot,
                                size_str,
                            });
                            self.status_message = Some(format!("✓ Snapshot saved: {}", file_name));
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

    /// Processes system tray close interception and queued [`TrayEvent`] commands.
    #[cfg(target_os = "windows")]
    fn handle_tray_events(&mut self, ctx: &egui::Context) -> bool {
        if ctx.input(|i| i.viewport().close_requested())
            && !self.exit_requested
            && !is_exit_requested()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        if is_exit_requested() && !self.exit_requested {
            self.exit_requested = true;
            self.stop_recording();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return true;
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
                TrayEvent::ToggleRecording => self.toggle_recording(),
                TrayEvent::TakeSnapshot => self.trigger_screenshot(),
                TrayEvent::OpenOutputDir => {
                    let _ = open::that(&self.config.output_directory);
                }
                TrayEvent::SaveInstantReplay => self.save_instant_replay(),
                TrayEvent::ExitApp => {
                    self.exit_requested = true;
                    self.stop_recording();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return true;
                }
            }
        }

        false
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

        #[cfg(target_os = "windows")]
        if self.handle_tray_events(&ctx) {
            return;
        }

        // Global hotkeys (F9 = Toggle Record, F10 = Snapshot)
        let (toggle_record, take_snapshot) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::F9),
                i.key_pressed(egui::Key::F10),
            )
        });

        if toggle_record {
            self.toggle_recording();
        }
        if take_snapshot {
            self.trigger_screenshot();
        }

        // Synchronize live HUD state and VU meter metrics
        self.hud_state.is_recording = self.pipeline.is_active();
        self.hud_state.is_paused = self.pipeline.is_paused();
        self.hud_state.elapsed = self.pipeline.elapsed();
        let (peak_l, peak_r) = self.pipeline.live_audio_peaks();
        self.hud_state.audio_peak_l = peak_l;
        self.hud_state.audio_peak_r = peak_r;

        egui::Panel::top("top_header").show(ui, |ui| {
            ui.add_space(4.0);
            self.render_top_header(ui, lang);
            ui.add_space(4.0);
        });

        egui::Panel::bottom("bottom_status_bar").show(ui, |ui| {
            ui.add_space(3.0);
            self.render_bottom_status_bar(ui, lang);
            ui.add_space(3.0);
        });

        if self.hud_state.is_recording {
            egui::Panel::bottom("hud_floating_panel").show(ui, |ui| {
                ui.add_space(4.0);
                if let Some(action) = render_floating_hud(ui, &self.hud_state, lang) {
                    match action {
                        HudAction::Pause => {
                            let _ = self.pipeline.pause();
                        }
                        HudAction::Resume => {
                            let _ = self.pipeline.resume();
                        }
                        HudAction::Stop => {
                            self.stop_recording();
                        }
                    }
                }
                ui.add_space(4.0);
            });
        }

        egui::CentralPanel::default().show(ui, |ui| match self.active_tab {
            AppTab::Dashboard => self.render_dashboard(ui, lang),
            AppTab::Settings => self.render_settings_modal(ui, lang),
        });

        if self.hud_state.is_recording {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
    }
}
