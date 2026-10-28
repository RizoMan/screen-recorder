//! Multi-threaded recording pipeline orchestrator applying Dependency Inversion (DIP).
//!
//! Conforms to `[SPEC-ARCH-001]`, `[CONTRACT-ARCH-001]`, and RAII cleanup guarantees.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use parking_lot::Mutex;
use rec_core::audio::AudioFrame;
use rec_core::avi_writer::{encode_frame_to_jpeg, AviWriter};
use rec_core::config::AppConfig;
use rec_core::frame::VideoFrame;
use crate::errors::{EngineError, ReplayError};
use crate::replay_buffer::InstantReplayBuffer;
use crate::traits::{AudioBackend, CaptureBackend, CaptureTarget};

/// State of the recording pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordingState {
    /// Pipeline is idle and ready to record.
    Idle,
    /// Pipeline is actively capturing and writing frames.
    Recording {
        /// Timestamp when recording started.
        started_at: Instant,
        /// Destination file path on disk.
        output_path: PathBuf,
    },
    /// Pipeline is paused.
    Paused {
        /// Total elapsed duration before pause.
        accumulated: Duration,
        /// Instant when pause began.
        paused_at: Instant,
        /// Destination file path on disk.
        output_path: PathBuf,
    },
}

/// Central orchestrator coordinating capture backends, lock-free queues, and encoding threads.
pub struct RecordingPipeline {
    config: AppConfig,
    active_config: Arc<Mutex<AppConfig>>,
    capture_backend: Option<Box<dyn CaptureBackend>>,
    audio_backend: Option<Box<dyn AudioBackend>>,
    state: Arc<Mutex<RecordingState>>,
    running_flag: Arc<AtomicBool>,
    replay_buffer: Arc<InstantReplayBuffer>,
    video_tx: Option<Sender<VideoFrame>>,
    audio_tx: Option<Sender<AudioFrame>>,
    worker_handle: Option<JoinHandle<()>>,
    active_output_path: Arc<Mutex<Option<PathBuf>>>,
    latest_audio_peaks: Arc<Mutex<(f32, f32)>>,
}

struct EncodeJob {
    seq: u64,
    frame: VideoFrame,
}

struct EncodedFrame {
    seq: u64,
    jpeg_bytes: Vec<u8>,
}

/// Dispatches an audio frame into the container with strict monotonic alignment to Video Frame 0 (T0).
///
/// Handles pre-roll trimming, volume scaling, silence gap filling for startup delays or dropouts,
/// and container format configuration.
fn write_synchronized_audio(
    w: &mut AviWriter,
    audio: &AudioFrame,
    video_t0: Duration,
    active_config: &Arc<Mutex<AppConfig>>,
) {
    let mut frame = audio.clone();
    let (vol_pct, sys_enabled) = {
        let cfg = active_config.lock();
        (cfg.system_audio_volume_percent, cfg.record_system_audio)
    };

    if !sys_enabled || vol_pct == 0 {
        frame.apply_gain(0.0);
    } else if vol_pct != 100 {
        let gain = vol_pct as f32 / 100.0;
        frame.apply_gain(gain);
    }

    let audio_pts = frame.pts.as_duration();
    let sample_rate = frame.sample_rate as f64;
    let channels = frame.channels as f64;
    let total_samples = frame.samples.len() as f64;
    let frame_duration = Duration::from_secs_f64(total_samples / (sample_rate * channels).max(1.0));
    let audio_end_pts = audio_pts + frame_duration;

    // Discard pre-roll audio that completely preceded the first video frame
    if audio_end_pts <= video_t0 {
        return;
    }

    w.set_audio_format(frame.sample_rate, frame.channels);

    // If audio started before video T0 but overlaps past T0, trim pre-roll samples
    if audio_pts < video_t0 {
        let pre_roll = video_t0 - audio_pts;
        let samples_to_drop = ((pre_roll.as_secs_f64() * sample_rate).round() as usize * frame.channels as usize)
            .min(frame.samples.len());
        frame.samples.drain(0..samples_to_drop);
        if frame.samples.is_empty() {
            return;
        }
    }

    // Audio timestamp mapped into container timeline (relative to Video Frame 0)
    let target_timeline_pts = if audio_pts >= video_t0 {
        audio_pts - video_t0
    } else {
        Duration::ZERO
    };

    let cur_dur = w.audio_duration();

    // If there is a silence gap between currently written audio and target timestamp
    // (e.g. initial audio startup delay or hardware dropout > 15ms), bridge it with exact silence
    if target_timeline_pts > cur_dur + Duration::from_millis(15) {
        let silence_gap = target_timeline_pts - cur_dur;
        let _ = w.write_audio_silence(silence_gap);
    }

    let _ = w.write_audio_frame(&frame);
}

impl RecordingPipeline {
    /// Creates a new inactive `RecordingPipeline` with default components.
    ///
    /// # Arguments
    ///
    /// * `config` - Initial application configuration.
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        let replay_duration = Duration::from_secs(config.instant_replay_duration_secs as u64);
        let active_config = Arc::new(Mutex::new(config.clone()));
        Self {
            config,
            active_config,
            capture_backend: None,
            audio_backend: None,
            state: Arc::new(Mutex::new(RecordingState::Idle)),
            running_flag: Arc::new(AtomicBool::new(false)),
            replay_buffer: Arc::new(InstantReplayBuffer::new(replay_duration)),
            video_tx: None,
            audio_tx: None,
            worker_handle: None,
            active_output_path: Arc::new(Mutex::new(None)),
            latest_audio_peaks: Arc::new(Mutex::new((0.0, 0.0))),
        }
    }

    /// Injects the concrete platform video capture backend (DIP).
    pub fn set_capture_backend(&mut self, backend: Box<dyn CaptureBackend>) {
        self.capture_backend = Some(backend);
    }

    /// Injects the concrete platform audio backend (DIP).
    pub fn set_audio_backend(&mut self, backend: Box<dyn AudioBackend>) {
        self.audio_backend = Some(backend);
    }

    /// Updates active application configuration.
    pub fn set_config(&mut self, config: AppConfig) {
        *self.active_config.lock() = config.clone();
        self.config = config;
    }

    /// Starts capturing video and audio, writing directly into the specified output path.
    ///
    /// # Arguments
    ///
    /// * `target` - Display, window, or custom coordinate area to record.
    /// * `output_path` - File path where recorded video will be saved.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::PipelineAlreadyRunning`] if recording is already active,
    /// or backend initialization errors.
    pub fn start(&mut self, target: CaptureTarget, output_path: PathBuf) -> Result<PathBuf, EngineError> {
        let mut state_guard = self.state.lock();
        if !matches!(*state_guard, RecordingState::Idle) {
            return Err(EngineError::PipelineAlreadyRunning);
        }

        // Ensure target directory exists
        if let Some(parent) = output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let (v_tx, v_rx): (Sender<VideoFrame>, Receiver<VideoFrame>) = bounded(120);
        let (a_tx, a_rx): (Sender<AudioFrame>, Receiver<AudioFrame>) = unbounded();

        let (job_tx, job_rx) = unbounded::<EncodeJob>();
        let (enc_tx, enc_rx) = unbounded::<EncodedFrame>();

        let num_workers = std::thread::available_parallelism()
            .map(|n| n.get().clamp(4, 8))
            .unwrap_or(4);

        let quality_crf = self.config.quality_crf;
        // Map CRF (0..51) to JPEG quality (35..80). Default CRF 23 -> quality 52.
        let jpeg_quality = (84u32.saturating_sub((quality_crf as u32 * 14) / 10)).clamp(35, 80) as u8;

        let mut encoder_handles = Vec::with_capacity(num_workers);
        for i in 0..num_workers {
            let j_rx = job_rx.clone();
            let e_tx = enc_tx.clone();
            if let Ok(h) = std::thread::Builder::new()
                .name(format!("rec-encoder-{}", i))
                .spawn(move || {
                    while let Ok(job) = j_rx.recv() {
                        let jpeg_bytes = encode_frame_to_jpeg(&job.frame, jpeg_quality).unwrap_or_default();
                        let _ = e_tx.send(EncodedFrame {
                            seq: job.seq,
                            jpeg_bytes,
                        });
                    }
                })
            {
                encoder_handles.push(h);
            }
        }
        drop(enc_tx);

        self.running_flag.store(true, Ordering::SeqCst);
        let running_clone = Arc::clone(&self.running_flag);
        let replay_clone = Arc::clone(&self.replay_buffer);
        let out_path_clone = output_path.clone();
        let active_path_slot = Arc::clone(&self.active_output_path);
        let audio_peaks_slot = Arc::clone(&self.latest_audio_peaks);
        let active_config_slot = Arc::clone(&self.active_config);
        let target_fps = self.config.fps;
        let replay_duration = self.config.instant_replay_duration_secs;

        *active_path_slot.lock() = Some(output_path.clone());

        // Dedicated pipeline processing and disk writing thread
        let handle = std::thread::Builder::new()
            .name("rec-pipeline-worker".to_string())
            .spawn(move || {
                let mut writer: Option<AviWriter> = None;
                let mut video_t0_pts: Option<Duration> = None;
                let mut start_time: Option<Instant> = None;
                let mut frames_written = 0u64;
                let mut seq_dispatched = 0u64;
                let mut seq_written = 0u64;
                let mut pending_encoded = BTreeMap::<u64, Vec<u8>>::new();
                let mut pending_audio = Vec::<AudioFrame>::new();

                while running_clone.load(Ordering::Relaxed) || !v_rx.is_empty() || seq_written < seq_dispatched {
                    // 1. Ingest newly captured video frames and dispatch to parallel encoder pool
                    while let Ok(frame) = v_rx.try_recv() {
                        if writer.is_none() {
                            if let Ok(mut w) = AviWriter::new(
                                &out_path_clone,
                                frame.descriptor.width,
                                frame.descriptor.height,
                                target_fps,
                            ) {
                                let t0 = frame.pts.as_duration();
                                video_t0_pts = Some(t0);

                                // Process any pre-buffered audio accumulated before video frame 0
                                for audio in pending_audio.drain(..) {
                                    write_synchronized_audio(
                                        &mut w,
                                        &audio,
                                        t0,
                                        &active_config_slot,
                                    );
                                }

                                writer = Some(w);
                                start_time = Some(Instant::now());
                            }
                        }

                        if replay_duration > 0 {
                            replay_clone.push_video_frame(frame.clone());
                        }

                        let seq = seq_dispatched;
                        seq_dispatched += 1;
                        let _ = job_tx.send(EncodeJob { seq, frame });
                    }

                    // 2. Process audio frames with real-time volume, gain scaling, and calibrated synchronization
                    while let Ok(audio) = a_rx.try_recv() {
                        let (peak_l, peak_r) = audio.calculate_peaks();
                        *audio_peaks_slot.lock() = (peak_l, peak_r);

                        if let Some(t0) = video_t0_pts {
                            if let Some(ref mut w) = writer {
                                write_synchronized_audio(
                                    w,
                                    &audio,
                                    t0,
                                    &active_config_slot,
                                );
                            }
                        } else {
                            pending_audio.push(audio.clone());
                        }

                        if replay_duration > 0 {
                            replay_clone.push_audio_frame(audio);
                        }
                    }

                    // 2b. Audio hardware (WASAPI) continuously streams real samples (0.0 during system silence).
                    // Sub-buffer alignment with video is handled atomically at stream finalization.
                    if a_rx.is_empty() {
                        let mut peaks = audio_peaks_slot.lock();
                        peaks.0 *= 0.9;
                        peaks.1 *= 0.9;
                    }

                    // 3. Receive finished encoded frames from parallel workers
                    while let Ok(enc) = enc_rx.try_recv() {
                        pending_encoded.insert(enc.seq, enc.jpeg_bytes);
                    }

                    // 4. Write in-order sequential frames to disk with continuous CFR lockstep pacing
                    while let Some(jpeg) = pending_encoded.remove(&seq_written) {
                        if let Some(ref mut w) = writer {
                            if !jpeg.is_empty() {
                                let target_frames = start_time.map_or(0, |t| (t.elapsed().as_secs_f64() * target_fps as f64).round() as u64);

                                while frames_written + 1 < target_frames {
                                    let _ = w.repeat_last_frame();
                                    frames_written += 1;
                                }

                                let _ = w.write_preencoded_frame(&jpeg);
                                frames_written += 1;
                            }
                        }
                        seq_written += 1;
                    }

                    // 5. Continuous CFR pacer for static screens:
                    // Only duplicate when ALL captured frames have finished encoding (no in-flight jobs),
                    // preventing premature duplicates and artificial frame inflation.
                    if seq_written == seq_dispatched {
                        if let Some(ref mut w) = writer {
                            let target_frames = start_time.map_or(0, |t| (t.elapsed().as_secs_f64() * target_fps as f64).round() as u64);

                            if target_frames > frames_written {
                                while frames_written < target_frames {
                                    let _ = w.repeat_last_frame();
                                    frames_written += 1;
                                }
                            }
                        }
                    }

                    std::thread::sleep(Duration::from_millis(1));
                }

                // Recording has stopped and v_rx is empty.
                // Drop job_tx so worker threads will exit when all dispatched jobs are processed.
                drop(job_tx);

                // Drain any remaining in-flight jobs from parallel workers until seq_written == seq_dispatched
                while seq_written < seq_dispatched {
                    if let Ok(enc) = enc_rx.recv() {
                        pending_encoded.insert(enc.seq, enc.jpeg_bytes);
                        while let Some(jpeg) = pending_encoded.remove(&seq_written) {
                            if let Some(ref mut w) = writer {
                                if !jpeg.is_empty() {
                                    let target_frames = start_time.map_or(0, |t| (t.elapsed().as_secs_f64() * target_fps as f64).round() as u64);
                                    while frames_written + 1 < target_frames {
                                        let _ = w.repeat_last_frame();
                                        frames_written += 1;
                                    }
                                    let _ = w.write_preencoded_frame(&jpeg);
                                    frames_written += 1;
                                }
                            }
                            seq_written += 1;
                        }
                    } else {
                        break;
                    }
                }

                // Drain any trailing audio frames from the audio queue
                while let Ok(audio) = a_rx.try_recv() {
                    if let Some(t0) = video_t0_pts {
                        if let Some(ref mut w) = writer {
                            write_synchronized_audio(
                                w,
                                &audio,
                                t0,
                                &active_config_slot,
                            );
                        }
                    }
                }

                // Final lockstep A/V synchronization:
                // Match video frame count to exact audio duration (or wall clock if no audio)
                if let Some(ref mut w) = writer {
                    let final_target_count = if w.audio_bytes_written() > 0 {
                        (w.audio_duration().as_secs_f64() * target_fps as f64).round() as u64
                    } else {
                        start_time.map_or(frames_written, |t| {
                            (t.elapsed().as_secs_f64() * target_fps as f64).round() as u64
                        })
                    };

                    while frames_written < final_target_count {
                        let _ = w.repeat_last_frame();
                        frames_written += 1;
                    }

                    // Also match audio duration to video duration if video slightly exceeds audio
                    let video_duration = Duration::from_nanos((frames_written * 1_000_000_000) / (target_fps as u64));
                    let audio_dur = w.audio_duration();
                    if w.audio_bytes_written() > 0 && video_duration > audio_dur {
                        let _ = w.write_audio_silence(video_duration - audio_dur);
                    }

                    let _ = w.finalize();
                }

                // Cleanly join worker threads
                for h in encoder_handles {
                    let _ = h.join();
                }
            })
            .map_err(|e| EngineError::Capture(crate::errors::CaptureError::Internal(e.to_string())))?;


        self.worker_handle = Some(handle);

        let session_epoch = Instant::now();

        // Configure shared epoch for zero-drift monotonic A/V sync
        if let Some(backend) = self.capture_backend.as_mut() {
            backend.set_epoch(session_epoch);
            backend.set_fps(self.config.fps);
        }
        if let Some(backend) = self.audio_backend.as_mut() {
            backend.set_epoch(session_epoch);
        }

        // Start audio capture first so audio hardware / WASAPI initializes concurrently
        if self.config.record_system_audio || self.config.record_microphone {
            if let Some(backend) = self.audio_backend.as_mut() {
                let _ = backend.start_audio_capture(a_tx.clone());
            }
        }

        // Start video capture backend with user-configured framerate
        if let Some(backend) = self.capture_backend.as_mut() {
            backend.start_capture(target, v_tx.clone())?;
        }

        self.video_tx = Some(v_tx);
        self.audio_tx = Some(a_tx);
        *state_guard = RecordingState::Recording {
            started_at: Instant::now(),
            output_path: output_path.clone(),
        };

        Ok(output_path)
    }

    /// Pauses an active recording session.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::PipelineNotRunning`] if not currently recording.
    pub fn pause(&mut self) -> Result<(), EngineError> {
        let mut state_guard = self.state.lock();
        match *state_guard {
            RecordingState::Recording { started_at, ref output_path } => {
                let accumulated = started_at.elapsed();
                *state_guard = RecordingState::Paused {
                    accumulated,
                    paused_at: Instant::now(),
                    output_path: output_path.clone(),
                };
                Ok(())
            }
            _ => Err(EngineError::PipelineNotRunning),
        }
    }

    /// Resumes a previously paused recording session.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::PipelineNotRunning`] if not paused.
    pub fn resume(&mut self) -> Result<(), EngineError> {
        let mut state_guard = self.state.lock();
        match *state_guard {
            RecordingState::Paused { accumulated, ref output_path, .. } => {
                let new_started_at = Instant::now() - accumulated;
                *state_guard = RecordingState::Recording {
                    started_at: new_started_at,
                    output_path: output_path.clone(),
                };
                Ok(())
            }
            _ => Err(EngineError::PipelineNotRunning),
        }
    }

    /// Stops and finalizes the recording session, halting all capture backends and flushing the file.
    ///
    /// # Returns
    ///
    /// Returns the finalized video file path on success.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] if backend teardown fails.
    pub fn stop(&mut self) -> Result<Option<PathBuf>, EngineError> {
        self.running_flag.store(false, Ordering::SeqCst);

        if let Some(backend) = self.capture_backend.as_mut() {
            let _ = backend.stop_capture();
        }

        if let Some(backend) = self.audio_backend.as_mut() {
            let _ = backend.stop_audio_capture();
        }

        self.video_tx = None;
        self.audio_tx = None;

        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }

        let saved_path = self.active_output_path.lock().take();

        let mut state_guard = self.state.lock();
        *state_guard = RecordingState::Idle;
        *self.latest_audio_peaks.lock() = (0.0, 0.0);

        Ok(saved_path)
    }

    /// Saves the recent frames buffered in memory to an Instant Replay video file.
    ///
    /// # Arguments
    ///
    /// * `dest_path` - File path where the replay will be saved.
    ///
    /// # Returns
    ///
    /// The saved video path on success.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Replay`] if no frames are currently buffered.
    pub fn save_instant_replay<P: AsRef<Path>>(&self, dest_path: P) -> Result<PathBuf, EngineError> {
        let frames = self.replay_buffer.export_video_snapshot()?;
        if frames.is_empty() {
            return Err(EngineError::Replay(ReplayError::EmptyBuffer));
        }

        if let Some(parent) = dest_path.as_ref().parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let first = &frames[0];
        let mut writer = AviWriter::new(
            dest_path.as_ref(),
            first.descriptor.width,
            first.descriptor.height,
            self.config.fps,
        )
        .map_err(|e| EngineError::Replay(ReplayError::SaveFailed(e.to_string())))?;

        for frame in &frames {
            writer
                .write_frame(frame)
                .map_err(|e| EngineError::Replay(ReplayError::SaveFailed(e.to_string())))?;
        }

        let audio_frames = self.replay_buffer.export_audio_snapshot();
        for audio in &audio_frames {
            let _ = writer.write_audio_frame(audio);
        }

        writer
            .finalize()
            .map_err(|e| EngineError::Replay(ReplayError::SaveFailed(e.to_string())))?;

        Ok(dest_path.as_ref().to_path_buf())
    }

    /// Returns the current total elapsed recording duration.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        let state_guard = self.state.lock();
        match *state_guard {
            RecordingState::Recording { started_at, .. } => started_at.elapsed(),
            RecordingState::Paused { accumulated, .. } => accumulated,
            RecordingState::Idle => Duration::ZERO,
        }
    }

    /// Checks if recording is active (running or paused).
    #[must_use]
    pub fn is_active(&self) -> bool {
        let state_guard = self.state.lock();
        !matches!(*state_guard, RecordingState::Idle)
    }

    /// Checks if recording is currently paused.
    ///
    /// Returns `true` if in the [`RecordingState::Paused`] state.
    #[must_use]
    #[allow(clippy::match_like_matches_macro)]
    pub fn is_paused(&self) -> bool {
        let state_guard = self.state.lock();
        match *state_guard {
            RecordingState::Paused { .. } => true,
            _ => false,
        }
    }

    /// Returns live peak levels `(left, right)` for audio meters.
    #[must_use]
    pub fn live_audio_peaks(&self) -> (f32, f32) {
        *self.latest_audio_peaks.lock()
    }

    /// Returns a reference to the circular instant replay buffer.
    #[must_use]
    pub fn replay_buffer(&self) -> &Arc<InstantReplayBuffer> {
        &self.replay_buffer
    }
}

impl Drop for RecordingPipeline {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rec_core::frame::Pts;

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
        assert!(dur >= expected_min && dur <= expected_max, "Duration was {:?}", dur);

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
}

