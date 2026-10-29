//! OpenDML AVI 2.0 container muxing and VLC compatibility tests for `rec-core::avi_writer`.
//!
//! Conforms to `[SPEC-CORE-001]` and `RULE-SAFE-001`.

#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};
use rec_core::audio::AudioFrame;
use rec_core::avi_writer::AviWriter;
use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
use rec_core::pixel_format::PixelFormat;

#[test]
fn test_avi_writer_creation_and_frame_writing() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_output_rec.avi");

    let mut writer = AviWriter::new(&test_file, 160, 120, 30).unwrap();
    let desc = FrameDescriptor::new(160, 120, PixelFormat::Rgba).unwrap();
    let raw_pixels = vec![200u8; desc.required_buffer_size()];
    let frame = VideoFrame::new(desc, Pts::from_nanos(0), raw_pixels).unwrap();

    assert!(writer.write_frame(&frame).is_ok());
    let written_bytes = writer.finalize().unwrap();
    assert!(written_bytes > 4096);
    assert!(test_file.exists());

    let _ = std::fs::remove_file(test_file);
}

#[test]
fn test_avi_writer_audio_and_video() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_av_rec_valid.avi");

    let mut writer = AviWriter::new(&test_file, 160, 120, 30).unwrap();
    let desc = FrameDescriptor::new(160, 120, PixelFormat::Rgba).unwrap();
    let raw_pixels = vec![180u8; desc.required_buffer_size()];
    let video_frame = VideoFrame::new(desc, Pts::from_nanos(0), raw_pixels).unwrap();

    // Write video frames
    for _ in 0..5 {
        assert!(writer.write_frame(&video_frame).is_ok());
    }

    // Write audio frames
    for _ in 0..5 {
        let audio_frame = AudioFrame::new(
            48_000,
            2,
            Pts::from_nanos(0),
            vec![0.1; 960], // 960 samples = 20ms of stereo
        )
        .unwrap();
        assert!(writer.write_audio_frame(&audio_frame).is_ok());
    }
    assert_eq!(writer.audio_chunks(), 5);

    let written_bytes = writer.finalize().unwrap();
    assert!(written_bytes > 4096);
    assert!(test_file.exists());

    let vlc_paths = [
        r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
        r"C:\Program Files\VideoLAN\VLC\vlc.exe",
    ];
    for vlc_path in &vlc_paths {
        if std::path::Path::new(vlc_path).exists() {
            let output = std::process::Command::new(vlc_path)
                .args(["-I", "dummy", "-vv", test_file.to_str().unwrap(), "vlc://quit"])
                .output()
                .unwrap();
            let log = String::from_utf8_lossy(&output.stderr);
            assert!(
                !log.contains("broken or missing index"),
                "Single-chunk AVI must NOT report broken or missing index"
            );
            break;
        }
    }

    let _ = std::fs::remove_file(test_file);
}

#[test]
fn test_avi_writer_repeat_last_frame() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_repeat_rec.avi");

    let mut writer = AviWriter::new(&test_file, 160, 120, 30).unwrap();
    let desc = FrameDescriptor::new(160, 120, PixelFormat::Rgba).unwrap();
    let raw_pixels = vec![200u8; desc.required_buffer_size()];
    let frame = VideoFrame::new(desc, Pts::from_nanos(0), raw_pixels).unwrap();

    assert!(writer.write_frame(&frame).is_ok());
    assert_eq!(writer.frame_count(), 1);

    for _ in 0..5 {
        assert!(writer.repeat_last_frame().is_ok());
    }
    assert_eq!(writer.frame_count(), 6);

    let written_bytes = writer.finalize().unwrap();
    assert!(written_bytes > 4096);
    assert!(test_file.exists());

    let _ = std::fs::remove_file(test_file);
}

#[test]
fn test_benchmark_1080p_encode() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_bench_1080p.avi");

    let mut writer = AviWriter::new(&test_file, 1920, 1080, 60).unwrap();
    let desc = FrameDescriptor::new(1920, 1080, PixelFormat::Bgra).unwrap();
    let raw_pixels = vec![128u8; desc.required_buffer_size()];
    let frame = VideoFrame::new(desc, Pts::from_nanos(0), raw_pixels).unwrap();

    let t0 = Instant::now();
    writer.write_frame(&frame).unwrap();
    let d1 = t0.elapsed();
    println!("1080p write_frame took: {:?}", d1);

    let _ = writer.finalize();
    let _ = std::fs::remove_file(test_file);
}

#[test]
fn test_avi_writer_audio_silence() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_silence_rec.avi");

    let mut writer = AviWriter::new(&test_file, 160, 120, 30).unwrap();
    let desc = FrameDescriptor::new(160, 120, PixelFormat::Rgba).unwrap();
    let raw_pixels = vec![120u8; desc.required_buffer_size()];
    let frame = VideoFrame::new(desc, Pts::from_nanos(0), raw_pixels).unwrap();

    writer.write_frame(&frame).unwrap();

    // Write 100ms of silence
    writer
        .write_audio_silence(Duration::from_millis(100))
        .unwrap();
    assert_eq!(writer.audio_chunks(), 1);
    assert!(writer.audio_duration() >= Duration::from_millis(99));

    let size = writer.finalize().unwrap();
    assert!(size > 4096);
    assert!(test_file.exists());
    let _ = std::fs::remove_file(test_file);
}

#[test]
fn test_avi_writer_opendml_avix_split() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_opendml_split.avi");

    let mut writer = AviWriter::new(&test_file, 160, 120, 30).unwrap();
    let desc = FrameDescriptor::new(160, 120, PixelFormat::Rgba).unwrap();
    let raw_pixels = vec![150u8; desc.required_buffer_size()];
    let frame = VideoFrame::new(desc, Pts::from_nanos(0), raw_pixels).unwrap();

    // Write initial frame in primary RIFF
    writer.write_frame(&frame).unwrap();
    assert_eq!(writer.riff_chunk_count(), 1);

    // Manually trigger OpenDML AVIX split
    writer.force_avix_split().unwrap();
    assert_eq!(writer.riff_chunk_count(), 2);

    // Write another frame in the second AVIX chunk
    writer.write_frame(&frame).unwrap();
    assert_eq!(writer.frame_count(), 2);

    let size = writer.finalize().unwrap();
    assert!(size > 4096);
    assert!(test_file.exists());

    // Read file contents to verify "AVIX", "indx", "ix00", and "dmlh" are all present
    let file_bytes = std::fs::read(&test_file).unwrap();
    assert!(file_bytes.windows(4).any(|w| w == b"AVIX"));
    assert!(file_bytes.windows(4).any(|w| w == b"dmlh"));
    assert!(file_bytes.windows(4).any(|w| w == b"indx"));
    assert!(file_bytes.windows(4).any(|w| w == b"ix00"));

    let _ = std::fs::remove_file(test_file);
}

#[test]
fn test_vlc_opendml_compatibility_validation() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_vlc_odml_compat.avi");

    let mut writer = AviWriter::new(&test_file, 320, 240, 30).unwrap();
    let desc = FrameDescriptor::new(320, 240, PixelFormat::Rgba).unwrap();
    let raw_pixels = vec![120u8; desc.required_buffer_size()];
    let frame = VideoFrame::new(desc, Pts::from_nanos(0), raw_pixels).unwrap();

    // Write frames and audio in primary RIFF chunk
    for _ in 0..10 {
        writer.write_frame(&frame).unwrap();
        let audio_frame =
            AudioFrame::new(48_000, 2, Pts::from_nanos(0), vec![0.05; 960]).unwrap();
        writer.write_audio_frame(&audio_frame).unwrap();
    }

    // Trigger OpenDML AVIX chunk split
    writer.force_avix_split().unwrap();

    // Write frames and audio in AVIX chunk
    for _ in 0..10 {
        writer.write_frame(&frame).unwrap();
        let audio_frame =
            AudioFrame::new(48_000, 2, Pts::from_nanos(0), vec![0.05; 960]).unwrap();
        writer.write_audio_frame(&audio_frame).unwrap();
    }

    let size = writer.finalize().unwrap();
    assert!(size > 4096);
    assert!(test_file.exists());

    // Check if VLC is installed on host system and test direct demuxer parsing
    let vlc_paths = [
        r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
        r"C:\Program Files\VideoLAN\VLC\vlc.exe",
    ];
    for vlc_path in &vlc_paths {
        if std::path::Path::new(vlc_path).exists() {
            let output = std::process::Command::new(vlc_path)
                .args(["-I", "dummy", "-vv", test_file.to_str().unwrap(), "vlc://quit"])
                .output()
                .unwrap();
            let log = String::from_utf8_lossy(&output.stderr);
            assert!(
                log.contains("detected OpenDML file"),
                "VLC should detect OpenDML file"
            );
            assert!(
                log.contains("selected ODML index for stream[0]"),
                "VLC should select ODML index for video"
            );
            assert!(
                !log.contains("broken or missing index"),
                "VLC should NOT report broken or missing index"
            );
            break;
        }
    }

    let _ = std::fs::remove_file(test_file);
}

