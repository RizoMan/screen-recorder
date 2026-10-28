//! Circular in-memory buffer for retroactive Instant Replay saving.
//!
//! Conforms to `[SPEC-FEAT-REPLAY-001]` and `[CONTRACT-REPLAY-001]`.

use std::collections::VecDeque;
use std::time::Duration;
use parking_lot::Mutex;
use rec_core::audio::AudioFrame;
use rec_core::frame::VideoFrame;
use crate::errors::ReplayError;

/// Circular in-memory buffer storing recent video and audio frames.
#[derive(Debug)]
pub struct InstantReplayBuffer {
    max_duration: Duration,
    video_frames: Mutex<VecDeque<VideoFrame>>,
    audio_frames: Mutex<VecDeque<AudioFrame>>,
}

impl InstantReplayBuffer {
    /// Creates a new `InstantReplayBuffer` with the specified maximum retention duration.
    ///
    /// # Arguments
    ///
    /// * `max_duration` - Maximum time window to retain (e.g. 30 seconds).
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
            audio_frames: Mutex::new(VecDeque::new()),
        }
    }

    /// Pushes a newly captured video frame into the buffer, evicting frames older than `max_duration`.
    ///
    /// # Arguments
    ///
    /// * `frame` - The new video frame.
    pub fn push_video_frame(&self, frame: VideoFrame) {
        let mut queue = self.video_frames.lock();
        let frame_pts = frame.pts.as_nanos();
        queue.push_back(frame);

        // Evict expired frames
        let cutoff = frame_pts.saturating_sub(self.max_duration.as_nanos() as u64);
        while let Some(front) = queue.front() {
            if front.pts.as_nanos() < cutoff {
                queue.pop_front();
            } else {
                break;
            }
        }
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

    /// Exports a snapshot clone of the currently buffered video frames.
    ///
    /// # Errors
    ///
    /// Returns [`ReplayError::EmptyBuffer`] if no video frames are currently buffered.
    pub fn export_video_snapshot(&self) -> Result<Vec<VideoFrame>, ReplayError> {
        let queue = self.video_frames.lock();
        if queue.is_empty() {
            return Err(ReplayError::EmptyBuffer);
        }
        Ok(queue.iter().cloned().collect())
    }

    /// Exports a snapshot clone of the currently buffered audio frames.
    #[must_use]
    pub fn export_audio_snapshot(&self) -> Vec<AudioFrame> {
        let queue = self.audio_frames.lock();
        queue.iter().cloned().collect()
    }

    /// Returns the number of video frames currently buffered.
    #[must_use]
    pub fn video_count(&self) -> usize {
        self.video_frames.lock().len()
    }

    /// Returns the number of audio frames currently buffered.
    #[must_use]
    pub fn audio_count(&self) -> usize {
        self.audio_frames.lock().len()
    }

    /// Clears all buffered frames.
    pub fn clear(&self) {
        self.video_frames.lock().clear();
        self.audio_frames.lock().clear();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rec_core::frame::{FrameDescriptor, Pts};
    use rec_core::pixel_format::PixelFormat;

    #[test]
    fn test_instant_replay_eviction() {
        let buffer = InstantReplayBuffer::new(Duration::from_millis(100));
        let desc = FrameDescriptor::new(10, 10, PixelFormat::Rgba).unwrap();
        let data = vec![0u8; desc.required_buffer_size()];

        let f1 = VideoFrame::new(desc.clone(), Pts::from_nanos(0), data.clone()).unwrap();
        let f2 = VideoFrame::new(desc.clone(), Pts::from_nanos(50_000_000), data.clone()).unwrap();
        let f3 = VideoFrame::new(desc.clone(), Pts::from_nanos(150_000_000), data).unwrap();

        buffer.push_video_frame(f1);
        buffer.push_video_frame(f2);
        assert_eq!(buffer.video_count(), 2);

        buffer.push_video_frame(f3);
        // f1 (0ns) should be evicted because 150ms - 100ms = 50ms cutoff
        assert_eq!(buffer.video_count(), 2);
    }
}
