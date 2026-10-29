//! CLI argument parsing and HUD duration formatting tests for `rec-app`.
//!
//! Conforms to `[SPEC-GUI-001]` and `RULE-SAFE-001`.

use std::time::Duration;
use rec_app::cli::CliOptions;
use rec_app::hud::format_duration;

#[test]
fn test_cli_parsing_record_and_area() {
    let args = ["--record", "--area", "10,20,1280,720", "--duration", "60"];
    let opts = CliOptions::parse_from(args);
    assert!(opts.record);
    assert_eq!(opts.area, Some((10, 20, 1280, 720)));
    assert_eq!(opts.duration_secs, Some(60));
}

#[test]
fn test_format_duration_seconds() {
    assert_eq!(format_duration(Duration::from_secs(5)), "00:05");
    assert_eq!(format_duration(Duration::from_secs(65)), "01:05");
    assert_eq!(format_duration(Duration::from_secs(3665)), "01:01:05");
}

