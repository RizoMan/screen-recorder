//! Circular in-memory buffer for retroactive Instant Replay saving.
//!
//! Conforms to `[SPEC-FEAT-REPLAY-001]` and `[CONTRACT-REPLAY-001]`.

use std::collections::VecDeque;
use std::time::Duration;
use parking_lot::Mutex;
use rec_core::audio::AudioFrame;
use rec_core::frame::{Pts, VideoFrame};
use crate::errors::ReplayError;

/// Strict hard ceiling on Instant Replay RAM usage (256 MB) to prevent out-of-memory errors
/// at high resolutions (1080p/1440p/4K) or long replay windows.
pub const MAX_REPLAY_MEMORY_BYTES: usize = 256 * 1024 * 1024;

/// Compressed Motion-JPEG video frame stored in the circular Instant Replay buffer.
#[derive(Debug, Clone)]
pub struct EncodedReplayFrame {
    /// Presentation timestamp of the frame.
    pub pts: Pts,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Pre-encoded JPEG payload (~80x smaller than raw RGBA/BGRA).
    pub jpeg_bytes: Vec<u8>,
}

/// Circular in-memory buffer storing recent video and audio frames.
#[derive(Debug)]
pub struct InstantReplayBuffer {
    max_duration: Duration,
    video_frames: Mutex<VecDeque<VideoFrame>>,
    encoded_frames: Mutex<VecDeque<EncodedReplayFrame>>,
    audio_frames: Mutex<VecDeque<AudioFrame>>,
}

/// Evicts items older than `max_duration` relative to `latest_pts_nanos` and enforces
/// [`MAX_REPLAY_MEMORY_BYTES`] on a bounded [`VecDeque`] (DRY helper).
fn evict_expired_and_cap_memory<T>(
    queue: &mut VecDeque<T>,
    latest_pts_nanos: u64,
    max_duration: Duration,
    pts_fn: impl Fn(&T) -> u64,
    bytes_fn: impl Fn(&T) -> usize,
) {
    let cutoff = latest_pts_nanos.saturating_sub(max_duration.as_nanos() as u64);
    while let Some(front) = queue.front() {
        if pts_fn(front) < cutoff {
            queue.pop_front();
        } else {
            break;
        }
    }

    let mut total_bytes: usize = queue.iter().map(&bytes_fn).sum();
    while total_bytes > MAX_REPLAY_MEMORY_BYTES && queue.len() > 1 {
        if let Some(removed) = queue.pop_front() {
            total_bytes = total_bytes.saturating_sub(bytes_fn(&removed));
        } else {
            break;
        }
    }
}

impl InstantReplayBuffer {
    /// Creates a new [`InstantReplayBuffer`] with the specified maximum retention duration.
    ///
    /// # Arguments
    ///
    /// * `max_duration` - Maximum time window to retain (e.g. 30 seconds).
    ///
    /// # Returns
    ///
    /// A newly initialized, empty [`InstantReplayBuffer`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use rec_engine::replay_buffer::InstantReplayBuffer;
    ///
    /// let buffer = InstantReplayBuffer::new(Duration::from_secs(30));
    /// assert_eq!(buffer.video_count(), 0);
    /// ```
    #[must_use]
    pub fn new(max_duration: Duration) -> Self {
        Self {
            max_duration,
            video_frames: Mutex::new(VecDeque::new()),
            encoded_frames: Mutex::new(VecDeque::new()),
            audio_frames: Mutex::new(VecDeque::new()),
        }
    }

    /// Pushes a newly captured video frame into the buffer, evicting frames older than `max_duration`
    /// or exceeding [`MAX_REPLAY_MEMORY_BYTES`].
    ///
    /// # Arguments
    ///
    /// * `frame` - The new video frame.
    pub fn push_video_frame(&self, frame: VideoFrame) {
        let mut queue = self.video_frames.lock();
        let frame_pts = frame.pts.as_nanos();
        queue.push_back(frame);
        evict_expired_and_cap_memory(
            &mut queue,
            frame_pts,
            self.max_duration,
            |f| f.pts.as_nanos(),
            |f| f.data.len(),
        );
    }

    /// Pushes a compressed JPEG video frame into the replay buffer, evicting expired frames
    /// or frames exceeding [`MAX_REPLAY_MEMORY_BYTES`].
    ///
    /// # Arguments
    ///
    /// * `frame` - Pre-encoded JPEG frame metadata and payload.
    pub fn push_encoded_frame(&self, frame: EncodedReplayFrame) {
        let mut queue = self.encoded_frames.lock();
        let frame_pts = frame.pts.as_nanos();
        queue.push_back(frame);
        evict_expired_and_cap_memory(
            &mut queue,
            frame_pts,
            self.max_duration,
            |f| f.pts.as_nanos(),
            |f| f.jpeg_bytes.len(),
        );
    }

    /// Pushes an audio frame into the buffer, evicting frames older than `max_duration`.
    ///
    /// # Arguments
    ///
    /// * `frame` - The new audio frame.
    pub fn push_audio_frame(&self, frame: AudioFrame) {
        let mut queue = self.audio_frames.lock();
        let frame_pts = frame.pts.as_nanos();
        queue.push_back(frame);

        let cutoff = frame_pts.saturating_sub(self.max_duration.as_nanos() as u64);
        while let Some(front) = queue.front() {
            if front.pts.as_nanos() < cutoff {
                queue.pop_front();
            } else {
                break;
            }
        }
    }

    /// Exports a snapshot clone of the currently buffered raw video frames.
    ///
    /// # Returns
    ///
    /// Ordered vector of buffered [`VideoFrame`] instances.
    ///
    /// # Errors
    ///
    /// Returns [`ReplayError::EmptyBuffer`] if no raw video frames are currently buffered.
    pub fn export_video_snapshot(&self) -> Result<Vec<VideoFrame>, ReplayError> {
        let queue = self.video_frames.lock();
        if queue.is_empty() {
            return Err(ReplayError::EmptyBuffer);
        }
        Ok(queue.iter().cloned().collect())
    }

    /// Exports a snapshot clone of the currently buffered compressed JPEG video frames.
    ///
    /// # Returns
    ///
    /// Ordered vector of [`EncodedReplayFrame`] instances currently in the circular buffer.
    #[must_use]
    pub fn export_encoded_snapshot(&self) -> Vec<EncodedReplayFrame> {
        let queue = self.encoded_frames.lock();
        queue.iter().cloned().collect()
    }

    /// Exports a snapshot clone of the currently buffered audio frames.
    ///
    /// # Returns
    ///
    /// Ordered vector of [`AudioFrame`] instances currently in the circular buffer.
    #[must_use]
    pub fn export_audio_snapshot(&self) -> Vec<AudioFrame> {
        let queue = self.audio_frames.lock();
        queue.iter().cloned().collect()
    }

    /// Returns the number of video frames currently buffered.
    ///
    /// # Returns
    ///
    /// Total count of raw and pre-encoded video frames in the buffer.
    #[must_use]
    pub fn video_count(&self) -> usize {
        let raw_len = self.video_frames.lock().len();
        let enc_len = self.encoded_frames.lock().len();
        raw_len + enc_len
    }

    /// Returns the number of audio frames currently buffered.
    ///
    /// # Returns
    ///
    /// Count of [`AudioFrame`] items in the buffer.
    #[must_use]
    pub fn audio_count(&self) -> usize {
        self.audio_frames.lock().len()
    }

    /// Clears all buffered frames and releases their heap memory.
    pub fn clear(&self) {
        let mut vf = self.video_frames.lock();
        vf.clear();
        vf.shrink_to_fit();

        let mut ef = self.encoded_frames.lock();
        ef.clear();
        ef.shrink_to_fit();

        let mut af = self.audio_frames.lock();
        af.clear();
        af.shrink_to_fit();
    }
}

