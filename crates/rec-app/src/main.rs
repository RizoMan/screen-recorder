#![windows_subsystem = "windows"]
//! `rec-app`: Native GUI and CLI application for Screen Recorder.
//!
//! Conforms to `[SPEC-GUI-001]` and `RULE-SAFE-001`.

pub mod cli;
pub mod gui;
pub mod hud;
pub mod theme;

use std::error::Error;
use eframe::egui::Vec2;
use rec_core::config::AppConfig;
use rec_engine::pipeline::RecordingPipeline;
use rec_platform_windows::{WasapiAudioBackend, WgcCaptureBackend};
use crate::cli::CliOptions;
use crate::gui::RecorderGuiApp;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // If CLI arguments were provided, attach output streams to caller terminal
    if !args.is_empty() {
        #[cfg(target_os = "windows")]
        rec_platform_windows::attach_parent_console();
    }

    let cli_opts = CliOptions::parse_from(&args);

    if cli_opts.record {
        println!("Screen Recorder Pro - CLI Mode");
        let mut config = AppConfig::new();
        cli_opts.apply_to_config(&mut config);

        let mut pipeline = RecordingPipeline::new(config.clone());
        #[cfg(target_os = "windows")]
        {
            pipeline.set_capture_backend(Box::new(WgcCaptureBackend::new()));
            pipeline.set_audio_backend(Box::new(WasapiAudioBackend::new()));
        }

        let target = cli_opts.to_capture_target();
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let output_path = cli_opts.output.clone().unwrap_or_else(|| {
            config.output_directory.join(format!("Recording_{}.avi", timestamp))
        });

        println!("Starting capture for target: {:?}", target);
        println!("Destination: {:?}", output_path);
        pipeline.start(target, output_path)?;

        let duration = cli_opts.duration_secs.unwrap_or(5);
        println!("Recording for {} seconds...", duration);
        std::thread::sleep(std::time::Duration::from_secs(duration));

        println!("Stopping recording and finalizing video container...");
        let saved = pipeline.stop()?;
        if let Some(path) = saved {
            println!("Recording successfully saved: {:?}", path);
        } else {
            println!("Recording stopped.");
        }
        return Ok(());
    }

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(860.0, 560.0))
            .with_min_inner_size(Vec2::new(720.0, 480.0))
            .with_title("Screen Recorder Pro"),
        ..Default::default()
    };

    eframe::run_native(
        "Screen Recorder Pro",
        native_options,
        Box::new(|cc| {
            theme::setup_custom_fonts(&cc.egui_ctx);
            theme::configure_obsidian_theme(&cc.egui_ctx);
            Ok(Box::new(RecorderGuiApp::new_with_context(Some(&cc.egui_ctx))))
        }),
    ).map_err(|e| format!("GUI launch failed: {}", e))?;

    Ok(())
}
