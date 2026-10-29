//! Pipeline A/V synchronization and Instant Replay circular buffer tests for `rec-engine`.
//!
//! Conforms to `[SPEC-ARCH-001]`, `[SPEC-FEAT-REPLAY-001]`, and `RULE-SAFE-001`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use rec_core::audio::AudioFrame;
use rec_core::avi_writer::AviWriter;
use rec_core::config::AppConfig;
use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
use rec_core::pixel_format::PixelFormat;
use rec_engine::pipeline::write_synchronized_audio;
use rec_engine::replay_buffer::{EncodedReplayFrame, InstantReplayBuffer};

#[test]
fn test_write_synchronized_audio_startup_delay() {
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("test_sync_startup_delay.avi");
    let mut writer = AviWriter::new(&path, 640, 480, 30).expect("writer creation");
    let active_config = Arc::new(Mutex::new(AppConfig::new()));

    let video_t0 = Duration::from_millis(100);
    // Audio frame arrives with a 1.100s startup delay (PTS = 1.200s, T0 = 0.100s => timeline offset 1.100s)
    let audio_pts = Pts::from_duration(Duration::from_millis(1200));
    let samples = vec![0.5f32; 960]; // 10ms of 48kHz stereo (480 frames * 2)
    let audio = AudioFrame::new(48000, 2, audio_pts, samples).expect("valid audio frame");

    write_synchronized_audio(&mut writer, &audio, video_t0, &active_config);

    // Container audio duration must equal 1.100s silence + 0.010s audio = 1.110s
    let dur = writer.audio_duration();
    let expected_min = Duration::from_millis(1105);
    let expected_max = Duration::from_millis(1115);
    assert!(
        dur >= expected_min && dur <= expected_max,
        "Duration was {:?}",
        dur
    );

    let _ = writer.finalize();
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_write_synchronized_audio_preroll_discard() {
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("test_sync_preroll.avi");
    let mut writer = AviWriter::new(&path, 640, 480, 30).expect("writer creation");
    let active_config = Arc::new(Mutex::new(AppConfig::new()));

    let video_t0 = Duration::from_millis(500);
    // Audio frame captured at 100ms, duration 10ms (ended at 110ms, well before video T0 500ms)
    let audio_pts = Pts::from_duration(Duration::from_millis(100));
    let samples = vec![0.5f32; 960];
    let audio = AudioFrame::new(48000, 2, audio_pts, samples).expect("valid audio frame");

    write_synchronized_audio(&mut writer, &audio, video_t0, &active_config);

    // Pre-roll audio before video T0 must be completely discarded
    assert_eq!(writer.audio_bytes_written(), 0);
    assert_eq!(writer.audio_duration(), Duration::ZERO);

    let _ = writer.finalize();
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_instant_replay_eviction() {
    let buffer = InstantReplayBuffer::new(Duration::from_millis(100));
    let desc = FrameDescriptor::new(10, 10, PixelFormat::Rgba).unwrap();
    let data = vec![0u8; desc.required_buffer_size()];

    let f1 = VideoFrame::new(desc.clone(), Pts::from_nanos(0), data.clone()).unwrap();
    let f2 = VideoFrame::new(desc.clone(), Pts::from_nanos(50_000_000), data.clone()).unwrap();
    let f3 = VideoFrame::new(desc, Pts::from_nanos(150_000_000), data).unwrap();

    buffer.push_video_frame(f1);
    buffer.push_video_frame(f2);
    assert_eq!(buffer.video_count(), 2);

    buffer.push_video_frame(f3);
    // f1 (0ns) should be evicted because 150ms - 100ms = 50ms cutoff
    assert_eq!(buffer.video_count(), 2);
}

#[test]
fn test_instant_replay_encoded_frames() {
    let buffer = InstantReplayBuffer::new(Duration::from_millis(100));
    buffer.push_encoded_frame(EncodedReplayFrame {
        pts: Pts::from_nanos(0),
        width: 1920,
        height: 1080,
        jpeg_bytes: vec![1, 2, 3],
    });
    buffer.push_encoded_frame(EncodedReplayFrame {
        pts: Pts::from_nanos(150_000_000),
        width: 1920,
        height: 1080,
        jpeg_bytes: vec![4, 5, 6],
    });
    assert_eq!(buffer.video_count(), 1);
    let snap = buffer.export_encoded_snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].jpeg_bytes, vec![4, 5, 6]);
}

