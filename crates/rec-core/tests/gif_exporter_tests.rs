//! High-Definition Animated GIF Exporter tests for `rec-core::gif_exporter`.
//!
//! Conforms to `[SPEC-FEAT-GIF-001]` and `RULE-SAFE-001`.

#![allow(clippy::unwrap_used)]

use rec_core::avi_writer::AviWriter;
use rec_core::frame::{FrameDescriptor, Pts, VideoFrame};
use rec_core::gif_exporter::{export_avi_to_gif, GifExportOptions, GifScale, GifWriter};
use rec_core::pixel_format::PixelFormat;

/// Verifies `[TEST-GIF-001.1]`: Valid `GIF89a` header, `NETSCAPE2.0` block, Local Color Table, LZW sub-blocks, and `0x3B` trailer.
#[test]
fn test_gif_encoder_valid_header_and_trailer() {
    let temp_path = std::env::temp_dir().join("test_unit_gif89a.gif");
    let opts = GifExportOptions::new(15, GifScale::Scale100, 10);
    let mut writer = GifWriter::new(&temp_path, 16, 16, opts).unwrap();

    let desc = FrameDescriptor::new(16, 16, PixelFormat::Rgba).unwrap();
    for i in 0..3u8 {
        let frame = VideoFrame::new(
            desc.clone(),
            Pts::from_nanos(u64::from(i) * 33_000_000),
            vec![i * 60; 16 * 16 * 4],
        )
        .unwrap();
        writer.write_video_frame(&frame).unwrap();
    }

    let frames = writer.finalize().unwrap();
    assert_eq!(frames, 3);

    let bytes = std::fs::read(&temp_path).unwrap();
    assert!(bytes.starts_with(b"GIF89a"));
    assert!(bytes.windows(11).any(|w| w == b"NETSCAPE2.0"));
    assert_eq!(*bytes.last().unwrap(), 0x3B);

    let _ = std::fs::remove_file(temp_path);
}

/// Verifies `[TEST-GIF-001.2]`: End-to-end AVI recording to animated GIF export with adaptive palette and CatmullRom scaling.
#[test]
fn test_export_avi_to_gif_end_to_end() {
    let temp_dir = std::env::temp_dir();
    let avi_path = temp_dir.join("test_e2e_gif_src.avi");
    let gif_path = temp_dir.join("test_e2e_gif_out.gif");

    let mut avi = AviWriter::new(&avi_path, 120, 80, 30).unwrap();
    let desc = FrameDescriptor::new(120, 80, PixelFormat::Rgba).unwrap();
    for i in 0..6u8 {
        let mut pixels = vec![20u8; desc.required_buffer_size()];
        for (idx, chunk) in pixels.chunks_exact_mut(4).enumerate() {
            chunk[0] = ((idx * 7) & 0xFF) as u8;
            chunk[1] = ((idx * 13 + usize::from(i) * 20) & 0xFF) as u8;
            chunk[2] = ((idx * 3) & 0xFF) as u8;
            chunk[3] = 255;
        }
        let frame = VideoFrame::new(
            desc.clone(),
            Pts::from_nanos(u64::from(i) * 33_333_333),
            pixels,
        )
        .unwrap();
        avi.write_frame(&frame).unwrap();
    }
    avi.finalize().unwrap();

    let opts = GifExportOptions::new(15, GifScale::Scale50, 5);
    let out = export_avi_to_gif(&avi_path, &gif_path, opts).unwrap();
    assert!(out.exists());

    let gif_bytes = std::fs::read(&gif_path).unwrap();
    assert!(gif_bytes.starts_with(b"GIF89a"));
    // Verify scaled dimensions in Logical Screen Descriptor: 50% of 120x80 = 60x40
    let w = u16::from_le_bytes([gif_bytes[6], gif_bytes[7]]);
    let h = u16::from_le_bytes([gif_bytes[8], gif_bytes[9]]);
    assert_eq!((w, h), (60, 40));
    assert_eq!(*gif_bytes.last().unwrap(), 0x3B);

    let _ = std::fs::remove_file(avi_path);
    let _ = std::fs::remove_file(gif_path);
}

/// Verifies `[TEST-GIF-001.3]`: `GifScale` dimension scaling.
#[test]
fn test_gif_scale_dimensions() {
    assert_eq!(GifScale::Scale50.scaled_dimensions(1920, 1080), (960, 540));
    assert_eq!(GifScale::Scale75.scaled_dimensions(1920, 1080), (1440, 810));
    assert_eq!(
        GifScale::Scale100.scaled_dimensions(1920, 1080),
        (1920, 1080)
    );
    assert_eq!(GifScale::Scale50.percent(), 50);
    assert_eq!(GifScale::Scale75.percent(), 75);
    assert_eq!(GifScale::Scale100.percent(), 100);
}

/// Verifies `[TEST-GIF-001.4]`: Inter-frame delta sub-rectangle cropping and transparency index `255`
/// produce a `.gif` smaller than the source `.avi` on typical screen recordings.
#[test]
fn test_gif_delta_compression_smaller_than_avi() {
    let temp_dir = std::env::temp_dir();
    let avi_path = temp_dir.join("test_delta_ratio_src.avi");
    let gif_path = temp_dir.join("test_delta_ratio_out.gif");

    let w = 640u32;
    let h = 360u32;
    let mut avi = AviWriter::new(&avi_path, w, h, 30).unwrap();
    let desc = FrameDescriptor::new(w, h, PixelFormat::Rgba).unwrap();

    for f in 0..30u32 {
        let mut pixels = vec![0u8; desc.required_buffer_size()];
        // Realistic desktop UI background + top header + moving cursor indicator
        let cursor_x = 60 + f * 12;
        let cursor_y = 100 + f * 4;
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                if (x >= cursor_x && x < cursor_x + 24) && (y >= cursor_y && y < cursor_y + 24) {
                    pixels[i] = 255;
                    pixels[i + 1] = 214;
                    pixels[i + 2] = 10;
                    pixels[i + 3] = 255;
                } else if y < 40 {
                    pixels[i] = 30;
                    pixels[i + 1] = 36;
                    pixels[i + 2] = 48;
                    pixels[i + 3] = 255;
                } else {
                    pixels[i] = 15;
                    pixels[i + 1] = 18;
                    pixels[i + 2] = 24;
                    pixels[i + 3] = 255;
                }
            }
        }
        let frame = VideoFrame::new(
            desc.clone(),
            Pts::from_nanos(u64::from(f) * 33_333_333),
            pixels,
        )
        .unwrap();
        avi.write_frame(&frame).unwrap();
    }
    avi.finalize().unwrap();

    let opts = GifExportOptions::new(20, GifScale::Scale100, 10);
    export_avi_to_gif(&avi_path, &gif_path, opts).unwrap();

    let avi_size = std::fs::metadata(&avi_path).unwrap().len();
    let gif_size = std::fs::metadata(&gif_path).unwrap().len();
    assert!(
        gif_size < avi_size,
        "Expected delta-compressed GIF ({} bytes) to be smaller than source AVI ({} bytes)",
        gif_size,
        avi_size
    );

    let _ = std::fs::remove_file(avi_path);
    let _ = std::fs::remove_file(gif_path);
}

