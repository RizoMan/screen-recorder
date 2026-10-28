# SPECIFICATION: CAPTURE DOMAIN MODELS, TIMESTAMPS & RING BUFFERS
Spec ID: `[SPEC-CORE-001]`
Status: APPROVED
Domain: Core Domain & Data Models

---

## 1. Overview
Defines fundamental data structures for video frames, pixel formats, audio sample buffers, presentation timestamps (PTS), and lock-free ring buffers in `rec-core`.

---

## 2. Requirements `[REQ-CORE-001]`
1. **Pixel Formats**: Support `Rgba`, `Nv12`, `I420`, and `Bgra` pixel formats with exact stride and byte calculation methods.
2. **Video Frames**: Represent uncompressed video frames (`VideoFrame`) with nanosecond PTS timestamps, width, height, stride, format, and memory buffer.
3. **Presentation Timestamps**: Monotonically non-decreasing timestamps (`Pts` and `Dts`) in nanoseconds or fractional time bases.
4. **Audio Frames**: Multi-channel PCM float32 audio buffers with sample rate and channel count metadata.
5. **Lock-Free Ring Buffer**: A Single-Producer Single-Consumer (SPSC) ring buffer enabling zero-copy or minimal-copy frame passing between capture and encoding threads.

---

## 3. Rust Contracts `[CONTRACT-CORE-001]`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba,
    Bgra,
    Nv12,
    I420,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pts(pub u64); // Nanoseconds

#[derive(Debug, Clone)]
pub struct FrameDescriptor {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
}

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub descriptor: FrameDescriptor,
    pub pts: Pts,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub sample_rate: u32,
    pub channels: u16,
    pub pts: Pts,
    pub samples: Vec<f32>,
}
```

---

## 4. Non-Functional Requirements `[NFR-CORE-001]`
1. Frame descriptor size calculations must be verified against integer overflows using checked arithmetic.
2. Ring buffer operations (`push`, `pop`) must be lock-free and bounded to prevent unbounded heap allocation.
3. No unsafe code in `rec-core` (`#![forbid(unsafe_code)]`).

---

## 5. Verification Criteria `[TEST-CORE-001]`
1. Unit tests verifying stride and buffer size calculation for all pixel formats.
2. Property-based tests verifying monotonic PTS arithmetic.
3. Concurrent SPSC ring buffer tests verifying lossless frame transfer across threads.
