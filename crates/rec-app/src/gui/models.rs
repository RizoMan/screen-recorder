//! Domain and presentation state models for the desktop GUI (`[SPEC-GUI-001]`).
//!
//! Applies the **Single Responsibility Principle (SRP)** and **Idiomatic Rust OOP** by isolating
//! strongly-typed view state machines (`enum`s) and media metadata descriptors from rendering logic.

use std::path::PathBuf;
use rec_core::i18n::{text, I18nKey, Language};
use rec_platform_windows::MonitorInfo;

/// Active view tab in the main application window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTab {
    /// Primary recording dashboard.
    #[default]
    Dashboard,
    /// Settings and preferences modal view.
    Settings,
}

/// Screen surface selection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureMode {
    /// Full screen monitor capture.
    #[default]
    FullScreen,
    /// Specific top-level application window.
    Window,
    /// Custom rectangular user-defined coordinates.
    CustomArea,
    /// Preset resolutions (1080p, 720p).
    Presets,
}

/// Settings sidebar category selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsCategory {
    /// Video encoder and hardware acceleration.
    #[default]
    VideoEncoder,
    /// Audio loopback and microphone channels.
    AudioChannels,
    /// Destination directory and container format.
    OutputFormat,
    /// Keyboard hotkeys.
    Hotkeys,
    /// Performance, cursor effects, and Instant Replay buffer.
    PerfBuffer,
}

/// Hardware encoder selection choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EncoderChoice {
    /// NVIDIA NVENC Gen 8.
    #[default]
    NvidiaNvenc,
    /// AMD AMF Core VCN 4.0.
    AmdAmf,
    /// Intel QuickSync Video QSV Gen12.
    IntelQuickSync,
    /// Multi-threaded CPU fallback.
    SoftwareCpu,
}

/// Bitrate rate control mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RateControlMode {
    /// Constant Bitrate.
    #[default]
    Cbr,
    /// Variable Bitrate.
    Vbr,
    /// Constant Quality Parameter.
    Cqp,
}

/// Video output container format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerFormat {
    /// High-performance Motion-JPEG OpenDML 2.0 AVI.
    #[default]
    Avi,
    /// Standard MP4 container (Phase 2 Roadmap).
    Mp4,
}

/// Strongly-typed classification of a recently captured media artifact.
///
/// Replaces fragile string-matching (`duration_str == "Snapshot"`) with an explicit Rust `enum`
/// in accordance with **KISS** and **RULE-ARCH-001**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecentMediaKind {
    /// Recorded video clip with formatted duration (`MM:SS min`).
    Video {
        /// Human-readable duration string.
        duration_str: String,
    },
    /// Static PNG screenshot capture.
    Snapshot,
    /// Exported animated GIF89a clip (`[SPEC-FEAT-GIF-001]`).
    AnimatedGif {
        /// Human-readable GIF parameters (e.g., `"15 FPS • 75%"`).
        spec_str: String,
    },
}

/// Metadata summary of the most recent recording, screenshot, or animated GIF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentRecordingMeta {
    /// Absolute path to the saved media file on disk.
    pub path: PathBuf,
    /// Classification of the media artifact (video clip, static snapshot, or animated GIF).
    pub kind: RecentMediaKind,
    /// Formatted file size string (e.g., `"14.2 MB"` or `"320.5 KB"`).
    pub size_str: String,
}

impl RecentRecordingMeta {
    /// Returns `true` if this media item is a static PNG screenshot.
    ///
    /// # Returns
    ///
    /// `true` when `self.kind` is [`RecentMediaKind::Snapshot`].
    #[must_use]
    pub const fn is_snapshot(&self) -> bool {
        match self.kind {
            RecentMediaKind::Snapshot => true,
            RecentMediaKind::Video { .. } | RecentMediaKind::AnimatedGif { .. } => false,
        }
    }

    /// Returns `true` if this media item is a recorded `.avi` video clip that can be exported to GIF.
    ///
    /// # Returns
    ///
    /// `true` when `self.kind` is [`RecentMediaKind::Video`].
    #[must_use]
    pub const fn is_video(&self) -> bool {
        match self.kind {
            RecentMediaKind::Video { .. } => true,
            RecentMediaKind::Snapshot | RecentMediaKind::AnimatedGif { .. } => false,
        }
    }

    /// Returns `true` if this media item is an exported animated `.gif`.
    ///
    /// # Returns
    ///
    /// `true` when `self.kind` is [`RecentMediaKind::AnimatedGif`].
    #[must_use]
    pub const fn is_gif(&self) -> bool {
        match self.kind {
            RecentMediaKind::AnimatedGif { .. } => true,
            RecentMediaKind::Video { .. } | RecentMediaKind::Snapshot => false,
        }
    }

    /// Returns the human-readable duration or type label for display in the UI.
    ///
    /// # Arguments
    ///
    /// * `lang` - Active display [`Language`] used to localize snapshot/GIF labels.
    ///
    /// # Returns
    ///
    /// Duration string for videos, GIF spec string for GIFs, or localized snapshot label for screenshots.
    #[must_use]
    pub fn display_duration(&self, lang: Language) -> &str {
        match self.kind {
            RecentMediaKind::Video { ref duration_str } => duration_str.as_str(),
            RecentMediaKind::Snapshot => text(I18nKey::SnapshotBadge, lang),
            RecentMediaKind::AnimatedGif { ref spec_str } => spec_str.as_str(),
        }
    }
}

/// Formats a monitor descriptor into a standardized display string (DRY helper).
///
/// # Arguments
///
/// * `monitor` - Reference to the [`MonitorInfo`] descriptor.
/// * `lang` - Current UI [`Language`] for localizing the primary badge.
///
/// # Returns
///
/// Formatted string such as `"\\.\DISPLAY1 [1920x1080] ★ Primary (sRGB)"`.
#[must_use]
pub fn format_monitor_label(monitor: &MonitorInfo, lang: Language) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(48);
    let _ = write!(
        &mut out,
        "\\\\.\\DISPLAY{} [{}x{}]",
        monitor.index + 1,
        monitor.width,
        monitor.height
    );
    if monitor.is_primary {
        out.push(' ');
        out.push_str(text(I18nKey::PrimaryBadge, lang));
    }
    out.push_str(" (sRGB)");
    out
}

/// Returns the localized fallback display label when no monitor descriptor is selected (`[REQ-GUI-001.1]`).
///
/// # Arguments
///
/// * `lang` - Current UI [`Language`] for localizing the primary badge.
///
/// # Returns
///
/// Formatted fallback display string.
#[must_use]
pub fn default_monitor_label(lang: Language) -> String {
    let mut s = String::from("\\\\.\\DISPLAY1 [1920x1080] ");
    s.push_str(text(I18nKey::PrimaryBadge, lang));
    s.push_str(" (sRGB)");
    s
}

/// Formats a byte count into a human-readable `KB` or `MB` string (DRY helper).
///
/// # Arguments
///
/// * `bytes` - File size in bytes.
///
/// # Returns
///
/// Human-readable file size string such as `"1.4 MB"` or `"240.0 KB"`.
#[must_use]
pub fn format_file_size(bytes: u64) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(16);
    if bytes >= 1_048_576 {
        let _ = write!(&mut out, "{:.1} MB", bytes as f64 / 1_048_576.0);
    } else {
        let _ = write!(&mut out, "{:.1} KB", bytes as f64 / 1024.0);
    }
    out
}
