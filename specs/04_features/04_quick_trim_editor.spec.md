# SPECIFICATION: Lossless Post-Recording Quick Trim Editor
Document ID: `[SPEC-FEAT-TRIM-001]`
Status: APPROVED / ACTIVE
Version: 0.1.0
Crate Scope: `rec-core`, `rec-app`

---

## 1. Purpose and Scope
This specification defines the domain models, zero-reencoding RIFF/OpenDML stream-copy remuxer, and interactive post-recording UI modal for trimming the start (**In Point**) and end (**Out Point**) of recorded `.avi` video clips in milliseconds without quality loss.

---

## 2. Functional Requirements (`[REQ-TRIM-001]`)

- **`[REQ-TRIM-001.1]` Fast AVI Container Probing (`probe_avi_metadata`)**:
  - Inspect recorded `.avi` containers in `rec-core::avi_trimmer` to extract video dimensions (`width`, `height`), frame rate (`fps`), total video chunk count (`total_frames`), and total playback duration in seconds (`duration_secs`) without decoding JPEG payloads.
- **`[REQ-TRIM-001.2]` Validated Trim Range Domain Model (`TrimRange`)**:
  - Encapsulate `start_sec` (**In Point**) and `end_sec` (**Out Point**) with strict invariant validation (`0.0 <= start_sec < end_sec <= duration_secs`, minimum clip duration `>= 0.2` seconds) and deterministic conversion to frame indices (`start_frame..end_frame`) and sample-aligned PCM byte offsets (`start_audio_byte..end_audio_byte`).
- **`[REQ-TRIM-001.3]` Zero-Reencoding Lossless Stream-Copy Remuxer (`trim_avi_lossless`)**:
  - Copy compressed `00dc` Motion-JPEG video chunks within `start_frame..end_frame` and synchronized `01wb` 16-bit PCM audio slices within `start_audio_byte..end_audio_byte` directly into a new OpenDML 2.0 + AVI 1.0 indexed `.avi` container via `AviWriter::write_preencoded_frame` and `AviWriter::write_raw_pcm_chunk`.
- **`[REQ-TRIM-001.4]` Interactive Quick Trim Modal & Bilingual i18n**:
  - Provide a **"✂ Trim"** action button on the Recent Recording card in the Dashboard that opens a dedicated modal (`crates/rec-app/src/gui/trim_modal.rs`) with interactive **In Point** and **Out Point** sliders, live trimmed duration badge, and one-click lossless export in both English and Spanish (`rec_core::i18n`).

---

## 3. Architectural Contracts (`[CONTRACT-TRIM-001]`)

```rust
/// Probed metadata of a recorded `.avi` video file (`[REQ-TRIM-001.1]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AviClipMetadata {
    /// Video width in pixels.
    pub width: u32,
    /// Video height in pixels.
    pub height: u32,
    /// Playback frame rate in frames per second.
    pub fps: u32,
    /// Total number of `00dc` video chunks in the container.
    pub total_frames: u32,
    /// Total clip duration in seconds.
    pub duration_secs: f64,
}

/// Validated In/Out trim range in seconds (`[REQ-TRIM-001.2]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimRange {
    /// Trim start timestamp in seconds (In Point).
    pub start_sec: f64,
    /// Trim end timestamp in seconds (Out Point).
    pub end_sec: f64,
}
```

---

## 4. Non-Functional Requirements (`[NFR-TRIM-001]`)

- **`[NFR-TRIM-001.1]` Zero Unsafe Code & Zero Generation Loss**: Must compile under `#![forbid(unsafe_code)]` and perform 100% lossless stream copy of JPEG video frames and PCM audio samples.
- **`[NFR-TRIM-001.2]` Bounded Memory Usage**: Stream chunk-by-chunk from the source `.avi` file without buffering the entire video in RAM.

---

## 5. Verification Matrix (`[TEST-TRIM-001]`)

| Test ID | Test Function | Verification Target |
| :--- | :--- | :--- |
| `[TEST-TRIM-001.1]` | `test_probe_avi_metadata_accuracy` | Verifies accurate extraction of dimensions, FPS, frame count, and duration from a recorded `.avi` file. |
| `[TEST-TRIM-001.2]` | `test_trim_range_validation_and_frame_mapping` | Verifies `TrimRange::new` sanitizes/validates bounds and computes exact frame and PCM byte ranges. |
| `[TEST-TRIM-001.3]` | `test_trim_avi_lossless_video_and_audio` | Records a 30-frame `.avi` with synchronized audio, trims `[0.3s..0.7s]`, and verifies exact frame count, playable headers, and reduced file size. |

