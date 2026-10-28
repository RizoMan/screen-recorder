# SPECIFICATION: INSTANT REPLAY CIRCULAR BUFFER
Spec ID: `[SPEC-FEAT-REPLAY-001]`
Status: APPROVED
Domain: High-Performance Engine Features

---

## 1. Overview
Defines an in-memory, fixed-capacity circular buffer that continuously stores the most recent $N$ seconds (e.g., 30s to 120s) of captured frames and audio packets, allowing instant retroactive clip saving at any moment.

---

## 2. Requirements `[REQ-REPLAY-001]`
1. **Configurable Duration**: Support buffer durations between 10 seconds and 300 seconds.
2. **Deterministic Memory Cap**: Allocate a fixed-size ring buffer for encoded packets or frame references to avoid memory growth.
3. **Atomic Flush/Save**: When the user triggers "Save Instant Replay" (via hotkey, GUI button, or HUD), the buffer atomically exports the retained timeline into a standalone container file (`Replay_YYYY-MM-DD_HH-MM-SS.mp4`) on a background I/O thread without interrupting live capture.
4. **Keyframe Alignment**: Ensure flush begins on a video keyframe (IDR frame) so that saved clips start cleanly without decoding artifacts.

---

## 3. Rust Contracts `[CONTRACT-REPLAY-001]`
```rust
pub struct InstantReplayBuffer {
    // Ring buffer of video packets & audio frames
}

impl InstantReplayBuffer {
    pub fn new(max_duration: std::time::Duration) -> Self;
    pub fn push_video_packet(&mut self, packet: EncodedPacket);
    pub fn push_audio_packet(&mut self, packet: EncodedAudioPacket);
    pub fn save_to_file(&self, destination: &std::path::Path) -> Result<std::path::PathBuf, ReplayError>;
}
```

---

## 4. Non-Functional Requirements `[NFR-REPLAY-001]`
1. Memory usage must stay strictly within the budgeted cap ($\text{Cap} = \text{Bitrate} \times \text{Duration} + \text{Overhead}$).
2. Triggering a replay save must not cause frame drops in active recording or background capture.

---

## 5. Verification Criteria `[TEST-REPLAY-001]`
1. Unit tests verifying circular eviction when buffer capacity is exceeded.
2. Timestamp continuity test verifying that flushed replay clips contain continuous non-decreasing PTS.
