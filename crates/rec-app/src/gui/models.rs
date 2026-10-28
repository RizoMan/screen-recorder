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
}

/// Metadata summary of the most recent recording or screenshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentRecordingMeta {
    /// Absolute path to the saved media file on disk.
    pub path: PathBuf,
    /// Classification of the media artifact (video clip vs. static snapshot).
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
        matches!(self.kind, RecentMediaKind::Snapshot)
    }

    /// Returns the human-readable duration or type label for display in the UI.
    ///
    /// # Arguments
    ///
    /// * `lang` - Active display [`Language`] used to localize snapshot labels.
    ///
    /// # Returns
    ///
    /// Duration string for videos, or localized snapshot label for screenshots.
    #[must_use]
    pub fn display_duration(&self, lang: Language) -> &str {
        match self.kind {
            RecentMediaKind::Video { ref duration_str } => duration_str.as_str(),
            RecentMediaKind::Snapshot => text(I18nKey::SnapshotBadge, lang),
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
    let primary_tag = if monitor.is_primary {
        format!(" {}", text(I18nKey::PrimaryBadge, lang))
    } else {
        String::new()
    };
    format!(
        "\\\\.\\DISPLAY{} [{}x{}]{} (sRGB)",
        monitor.index + 1,
        monitor.width,
        monitor.height,
        primary_tag
    )
}
