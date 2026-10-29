//! Lossless Post-Recording Quick Trim Editor tests for `rec-core::avi_trimmer`.
//!
//! Conforms to `[SPEC-FEAT-TRIM-001]` and `RULE-SAFE-001`.

#![allow(clippy::unwrap_used)]

use rec_core::audio::AudioFrame;
use rec_core::avi_trimmer::{probe_avi_metadata, trim_avi_lossless, TrimRange};
use rec_core::avi_writer::AviWriter;
use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
use rec_core::pixel_format::PixelFormat;

/// Verifies `[TEST-TRIM-001.1]`: Accurate probing of AVI dimensions, FPS, frame count, and duration.
#[test]
fn test_probe_avi_metadata_accuracy() {
    let temp_path = std::env::temp_dir().join("test_probe_avi_meta.avi");
    let mut writer = AviWriter::new(&temp_path, 240, 160, 25).unwrap();
    let desc = FrameDescriptor::new(240, 160, PixelFormat::Rgba).unwrap();
    for i in 0..50u64 {
        let frame = VideoFrame::new(
            desc.clone(),
            Pts::from_nanos(i * 40_000_000),
            vec![110u8; 240 * 160 * 4],
        )
        .unwrap();
        writer.write_frame(&frame).unwrap();
    }
    writer.finalize().unwrap();

    let meta = probe_avi_metadata(&temp_path).unwrap();
    assert_eq!(meta.width, 240);
    assert_eq!(meta.height, 160);
    assert_eq!(meta.fps, 25);
    assert_eq!(meta.total_frames, 50);
    assert!((meta.duration_secs - 2.0).abs() < 1e-5);

    let _ = std::fs::remove_file(temp_path);
}

/// Verifies `[TEST-TRIM-001.2]`: `TrimRange` invariant clamping and frame/audio byte bounds.
#[test]
fn test_trim_range_validation_and_frame_mapping() {
    let range = TrimRange::new(-2.0, 12.0, 10.0);
    assert_eq!(range.start_sec, 0.0);
    assert_eq!(range.end_sec, 10.0);

    let inverted = TrimRange::new(8.0, 3.0, 10.0);
    assert!(inverted.end_sec >= inverted.start_sec + TrimRange::MIN_DURATION_SECS);

    let sub = TrimRange::new(1.0, 2.5, 5.0);
    assert_eq!(sub.frame_bounds(30, 150), (30, 75));
    assert_eq!(sub.audio_byte_bounds(48_000, 2), (192_000, 480_000));
}

/// Verifies `[TEST-TRIM-001.3]`: Lossless stream copy of video (`00dc`) and audio (`01wb`) chunks.
#[test]
fn test_trim_avi_lossless_video_and_audio() {
    let temp_dir = std::env::temp_dir();
    let src_path = temp_dir.join("test_trim_lossless_src.avi");
    let dst_path = temp_dir.join("test_trim_lossless_dst.avi");

    let mut writer = AviWriter::new(&src_path, 160, 120, 10).unwrap();
    let desc = FrameDescriptor::new(160, 120, PixelFormat::Rgba).unwrap();
    // Write 30 frames (3.0 seconds at 10 FPS) + 3.0 seconds of stereo 48kHz audio
    for i in 0..30u64 {
        let frame = VideoFrame::new(
            desc.clone(),
            Pts::from_nanos(i * 100_000_000),
            vec![(i as u8) * 5; 160 * 120 * 4],
        )
        .unwrap();
        writer.write_frame(&frame).unwrap();

        let audio = AudioFrame::new(
            48_000,
            2,
            Pts::from_nanos(i * 100_000_000),
            vec![0.25f32; 4800 * 2], // 0.1s stereo at 48kHz
        )
        .unwrap();
        writer.write_audio_frame(&audio).unwrap();
    }
    writer.finalize().unwrap();

    // Trim to [1.0s .. 2.0s] -> should contain exactly 10 frames (1.0s duration)
    let range = TrimRange::new(1.0, 2.0, 3.0);
    let out = trim_avi_lossless(&src_path, &dst_path, range).unwrap();
    assert!(out.exists());

    let trimmed_meta = probe_avi_metadata(&dst_path).unwrap();
    assert_eq!(trimmed_meta.width, 160);
    assert_eq!(trimmed_meta.height, 120);
    assert_eq!(trimmed_meta.fps, 10);
    assert_eq!(trimmed_meta.total_frames, 10);
    assert!((trimmed_meta.duration_secs - 1.0).abs() < 1e-5);

    let src_size = std::fs::metadata(&src_path).unwrap().len();
    let dst_size = std::fs::metadata(&dst_path).unwrap().len();
    assert!(dst_size < src_size);

    let _ = std::fs::remove_file(src_path);
    let _ = std::fs::remove_file(dst_path);
}

