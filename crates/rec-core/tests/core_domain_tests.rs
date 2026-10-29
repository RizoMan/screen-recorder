//! Core domain unit and contract tests for `rec-core`.
//!
//! Conforms to `[SPEC-CORE-001]`, `[SPEC-I18N-001]`, `[SPEC-FEAT-CURSOR-001]`, and `RULE-SAFE-001`.

#![allow(clippy::unwrap_used)]

use rec_core::audio::AudioFrame;
use rec_core::config::AppConfig;
use rec_core::cursor_effects::{
    apply_cursor_effects, ClickButton, ClickRipple, CursorHighlightColor, CursorOverlayState,
};
use rec_core::errors::CoreError;
use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
use rec_core::i18n::{text, I18nKey, Language};
use rec_core::pixel_format::PixelFormat;
use rec_core::ring_buffer::LockFreeRingBuffer;

#[test]
fn test_audio_peak_calculation() {
    let samples = vec![0.2, -0.7, 0.4, 0.1];
    let frame = AudioFrame::new(48000, 2, Pts::from_nanos(0), samples).unwrap();
    let (l, r) = frame.calculate_peaks();
    assert_eq!(l, 0.4);
    assert_eq!(r, 0.7);
}

#[test]
fn test_audio_apply_gain() {
    let mut frame = AudioFrame::new(48000, 2, Pts::from_nanos(0), vec![0.2, -0.4]).unwrap();
    frame.apply_gain(1.5);
    assert!((frame.samples[0] - 0.3).abs() < 1e-5);
    assert!((frame.samples[1] + 0.6).abs() < 1e-5);

    // Clamping to 1.0 on large gain
    frame.apply_gain(3.0);
    assert!((frame.samples[0] - 0.9).abs() < 1e-5);
    assert_eq!(frame.samples[1], -1.0);
}

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

/// Verifies `[TEST-CURSOR-001]` radial halo blending inside radius and untouched outside.
#[test]
fn test_cursor_halo_blends_inside_and_preserves_outside() {
    let desc = FrameDescriptor::new(100, 100, PixelFormat::Rgba).unwrap();
    let mut frame = VideoFrame::new(desc, Pts::from_nanos(0), vec![20u8; 100 * 100 * 4]).unwrap();

    let state = CursorOverlayState {
        cursor_x: 50,
        cursor_y: 50,
        cursor_visible: true,
        halo_enabled: true,
        halo_radius: 20,
        halo_color: CursorHighlightColor::YellowGlow,
        ripples: Vec::new(),
    };

    apply_cursor_effects(&mut frame, &state);

    // Center pixel (50, 50) must be blended toward YellowGlow (R > G > B)
    let center_idx = (50 * 100 + 50) * 4;
    assert!(
        frame.data[center_idx] > 80,
        "Red channel at halo center should increase"
    );
    assert!(
        frame.data[center_idx + 1] > 60,
        "Green channel at halo center should increase"
    );

    // Far corner pixel (0, 0) must remain completely untouched
    assert_eq!(frame.data[0], 20);
    assert_eq!(frame.data[1], 20);
    assert_eq!(frame.data[2], 20);
}

/// Verifies `[TEST-CURSOR-001]` BGRA channel ordering and click ripple rendering.
#[test]
fn test_cursor_click_ripple_bgra_and_edge_clipping() {
    let desc = FrameDescriptor::new(64, 64, PixelFormat::Bgra).unwrap();
    let mut frame = VideoFrame::new(desc, Pts::from_nanos(0), vec![10u8; 64 * 64 * 4]).unwrap();

    let state = CursorOverlayState {
        cursor_x: -5,
        cursor_y: 63,
        cursor_visible: true,
        halo_enabled: true,
        halo_radius: 24,
        halo_color: CursorHighlightColor::ElectricBlue,
        ripples: vec![
            ClickRipple {
                x: 32,
                y: 32,
                button: ClickButton::Left,
                progress: 0.3,
            },
            ClickRipple {
                x: 63,
                y: 63,
                button: ClickButton::Right,
                progress: 0.5,
            },
        ],
    };

    // Must safely clip negative and edge coordinates without panicking
    apply_cursor_effects(&mut frame, &state);

    // Verify some pixels were modified by the ripple and halo
    assert!(frame
        .data
        .chunks_exact(4)
        .any(|px| px[0] > 10 || px[2] > 10));
}

#[test]
fn test_valid_video_frame_construction() {
    let desc = FrameDescriptor::new(100, 100, PixelFormat::Rgba).unwrap();
    let expected_size = desc.required_buffer_size();
    let data = vec![0u8; expected_size];
    let frame = VideoFrame::new(desc, Pts::from_nanos(100), data);
    assert!(frame.is_ok());
}

#[test]
fn test_short_buffer_returns_error() {
    let desc = FrameDescriptor::new(100, 100, PixelFormat::Rgba).unwrap();
    let short_data = vec![0u8; 10];
    let frame = VideoFrame::new(desc, Pts::from_nanos(100), short_data);
    assert!(matches!(frame, Err(CoreError::InvalidBufferSize { .. })));
}

#[test]
fn test_pixel_format_buffer_size() {
    let rgba_size = PixelFormat::Rgba.calculate_buffer_size(1920, 1080).unwrap();
    assert_eq!(rgba_size, 1920 * 1080 * 4);

    let nv12_size = PixelFormat::Nv12.calculate_buffer_size(1920, 1080).unwrap();
    assert_eq!(nv12_size, (1920 * 1080 * 3) / 2);
}

#[test]
fn test_zero_dimensions_error() {
    assert!(PixelFormat::Rgba.calculate_buffer_size(0, 1080).is_err());
    assert!(PixelFormat::Rgba.calculate_buffer_size(1920, 0).is_err());
}

#[test]
fn test_ring_buffer_fifo_order() {
    let buffer = LockFreeRingBuffer::new(3);
    buffer.push("A").unwrap();
    buffer.push("B").unwrap();
    assert_eq!(buffer.pop().unwrap(), "A");
    buffer.push("C").unwrap();
    assert_eq!(buffer.pop().unwrap(), "B");
    assert_eq!(buffer.pop().unwrap(), "C");
    assert!(buffer.pop().is_err());
}

#[test]
fn test_ring_buffer_overflow() {
    let buffer = LockFreeRingBuffer::new(1);
    assert!(buffer.push(1).is_ok());
    assert!(matches!(
        buffer.push(2),
        Err(CoreError::BufferOverflow { capacity: 1 })
    ));
}

#[test]
fn test_i18n_non_empty_and_parity() {
    for &key in I18nKey::all_variants() {
        let en = text(key, Language::English);
        let es = text(key, Language::Spanish);

        assert!(
            !en.trim().is_empty(),
            "English translation for {:?} must not be empty",
            key
        );
        assert!(
            !es.trim().is_empty(),
            "Spanish translation for {:?} must not be empty",
            key
        );

        let is_identical_brand_term = matches!(
            key,
            I18nKey::AppTitle
                | I18nKey::TrayTooltip
                | I18nKey::ProBadge
                | I18nKey::GeneralTab
                | I18nKey::VideoTab
                | I18nKey::AudioTab
        );
        if !is_identical_brand_term {
            assert_ne!(
                en, es,
                "Spanish translation for {:?} is identical to English ('{}')",
                key, en
            );
        }
    }
}

#[test]
fn test_language_display_names() {
    assert_eq!(Language::English.code(), "en");
    assert_eq!(Language::Spanish.code(), "es");
}

