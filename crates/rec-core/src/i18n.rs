//! Zero-overhead bilingual internationalization (i18n) system.
//!
//! Conforms to `[SPEC-I18N-001]`, `[CONTRACT-I18N-001]`, and `RULE-I18N-001`.

use serde::{Deserialize, Serialize};

/// Supported user interface and CLI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Language {
    /// English language (default).
    #[default]
    English,
    /// Spanish language.
    Spanish,
}

impl Language {
    /// Returns the ISO 639-1 language code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Spanish => "es",
        }
    }

    /// Returns the human-readable native language name.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
        }
    }
}

/// Exhaustive enumeration of all localized text keys used throughout the system.
///
/// Every key added here MUST have translations in both English and Spanish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum I18nKey {
    /// Main application window title.
    AppTitle,
    /// Application tagline / subtitle.
    AppSubtitle,
    /// Button label to begin recording.
    StartRecording,
    /// Button label to stop and finalize recording.
    StopRecording,
    /// Button label to pause an ongoing recording.
    PauseRecording,
    /// Button label to resume a paused recording.
    ResumeRecording,
    /// Button label to save instant replay buffer to disk.
    InstantReplaySave,
    /// Selection tool for custom rectangular area.
    SelectArea,
    /// Selection tool for specific window.
    SelectWindow,
    /// Selection tool for entire monitor.
    SelectMonitor,
    /// HUD label for elapsed recording duration.
    HudRecordingTime,
    /// HUD label for live audio meter.
    HudAudioLevel,
    /// Title of the settings dialog.
    SettingsTitle,
    /// Settings general tab title.
    GeneralTab,
    /// Settings video configuration tab.
    VideoTab,
    /// Settings audio configuration tab.
    AudioTab,
    /// Output folder destination setting label.
    OutputDirectory,
    /// Target framerate setting label.
    FrameRateLabel,
    /// Video resolution setting label.
    ResolutionLabel,
    /// Toggle label for GPU hardware encoding.
    HardwareAcceleration,
    /// Idle status badge text.
    StatusIdle,
    /// Active recording status badge text.
    StatusRecording,
    /// Paused status badge text.
    StatusPaused,
    /// Finalizing and saving status badge text.
    StatusSaving,
    /// Error message when OS access is denied.
    ErrorPermissionDenied,
    /// Error message when GPU device is lost or disconnected.
    ErrorDeviceLost,
    /// Error message when target monitor or window is unavailable.
    ErrorTargetNotFound,
    /// Generic unknown error label.
    ErrorUnknown,
    /// Button to open recordings directory.
    /// Button to open recordings directory.
    OpenRecordingsFolder,
    /// Context menu option to quit the app.
    ExitApp,
    /// Context menu option to open the recorder window.
    TrayOpen,
    /// System tray icon tooltip text.
    TrayTooltip,
    /// Tooltip or button to minimize to tray.
    TrayMinimizeToTray,
    /// PRO badge label in header.
    ProBadge,
    /// Engine architecture pill in header.
    EnginePill,
    /// Hardware ready badge pill.
    HwReadyPill,
    /// Mode tab for Full Screen monitor capture.
    ModeFullScreen,
    /// Mode tab for single window capture.
    ModeWindow,
    /// Mode tab for freeform custom area.
    ModeFreeRegion,
    /// Mode tab for resolution presets.
    ModePresets,
    /// Label for Primary Display section.
    PrimaryDisplayLabel,
    /// Label for active capture backend.
    CaptureBackendLabel,
    /// Label for System Audio source card.
    SystemAudioLabel,
    /// Label for Microphone source card.
    MicrophoneLabel,
    /// Label for Webcam Picture-in-Picture card.
    WebcamPipLabel,
    /// Label for Hardware Encoder badge.
    HwEncoderLabel,
    /// Label for destination path row.
    DestinationFolder,
    /// Button label to browse and change output path.
    BtnChange,
    /// Button label to open output folder in Explorer.
    BtnOpenFolder,
    /// Action button label to Start Recording with F9 hotkey.
    BtnStartRecordingF9,
    /// Action button label to Stop Recording with F9 hotkey.
    BtnStopRecordingF9,
    /// Action button label to Take Screenshot with F10 hotkey.
    BtnCaptureImageF10,
    /// Badge label for most recent recording card.
    RecentRecording,
    /// Button label to play the recent video.
    BtnPlay,
    /// Button label to show file in explorer.
    BtnShowInFolder,
    /// Bottom status bar message when engine is idle.
    EngineReady,
    /// Settings modal title for encoder setup.
    SettingsEncoderTitle,
    /// Settings sidebar tab for Video & Encoder.
    SettingsTabVideoEncoder,
    /// Settings sidebar tab for Audio & Channels.
    SettingsTabAudioChannels,
    /// Settings sidebar tab for Output & Format.
    SettingsTabOutputFormat,
    /// Settings sidebar tab for Hotkeys.
    SettingsTabHotkeys,
    /// Settings sidebar tab for Performance & Buffer.
    SettingsTabPerfBuffer,
    /// Section header for hardware encoder backends.
    HwEncoderBackendSection,
    /// Badge text for recommended encoder.
    RecommendedBadge,
    /// Label for detected GPU hardware.
    DetectedGpuLabel,
    /// Section header for Bitrate & Quality control.
    BitrateSectionTitle,
    /// Label for Bitrate Rate Control Mode.
    BitrateControlMode,
    /// Label for target bitrate slider.
    TargetBitrateLabel,
    /// Label for encoder preset selector.
    NvencPresetLabel,
    /// Label for encoding profile.
    ProfileLabel,
    /// Label for keyframe interval (GOP).
    KeyframesLabel,
    /// Metric label for pipeline engine latency.
    LatencyMetric,
    /// Metric label for dropped frames count.
    DroppedFramesMetric,
    /// Metric label for VRAM memory usage.
    VramMetric,
    /// Button label to reset settings to defaults.
    BtnResetDefaults,
    /// Button label to cancel changes.
    BtnCancel,
    /// Button label to save changes.
    BtnSave,
    /// Navigation tab label for Dashboard.
    NavDashboard,
    /// Navigation tab label for Settings.
    NavSettings,
    /// Primary monitor badge tag.
    PrimaryBadge,
    /// Label for UI font and element scale slider.
    UiScaleLabel,
    /// Preset button for compact UI scale.
    UiScaleSmall,
    /// Preset button for default 100% UI scale.
    UiScaleNormal,
    /// Preset button for large UI scale.
    UiScaleLarge,
    /// Preset button for extra large UI scale.
    UiScaleExtraLarge,
    /// Button label to copy media path to clipboard.
    BtnCopyPath,
    /// Button label to delete the selected recording/snapshot file.
    BtnDelete,
    /// Status notification when path is copied to clipboard.
    StatusCopiedToClipboard,
    /// Status notification when file is removed from disk.
    StatusFileDeleted,
    /// Tray context menu option to toggle recording with F9.
    TrayStartStopRecording,
    /// Tray context menu option to capture screenshot with F10.
    TrayTakeSnapshot,
    /// Tray context menu option to open the recordings output folder.
    TrayOpenFolder,
    /// Label for system audio volume level slider.
    SystemAudioVolumeLabel,
    /// Label for microphone gain level slider.
    MicrophoneGainLabel,
    /// Action button label to mute microphone.
    BtnMute,
    /// Action button label to unmute microphone.
    BtnUnmute,
    /// Technical label for audio sample rate and format specs.
    AudioSampleRateLabel,
    /// Detailed description for audio format specs.
    AudioSpecsDescription,
    /// Section title for Smart Cursor Effects in settings and dashboard.
    CursorEffectsSection,
    /// Toggle label for translucent cursor highlight halo.
    CursorHaloToggle,
    /// Toggle label for expanding mouse click ripple animations.
    CursorRipplesToggle,
    /// Slider label for cursor halo radius in pixels.
    CursorHaloRadiusLabel,
    /// Selector label for cursor halo highlight color.
    CursorColorLabel,
    /// Color preset label: Golden Yellow Glow.
    CursorColorYellow,
    /// Color preset label: Electric Blue.
    CursorColorBlue,
    /// Color preset label: Neon Emerald.
    CursorColorNeon,
    /// Badge label for screenshot snapshot items.
    SnapshotBadge,
    /// Button label to close the settings modal.
    BtnCloseModal,
    /// Informational notice when MP4 container is selected.
    Mp4RoadmapNotice,
    /// Hotkey description for starting/stopping recording.
    HotkeyStartStopLabel,
    /// Hotkey description for taking a screenshot.
    HotkeySnapshotLabel,
    /// Hotkey description for saving instant replay.
    HotkeyReplayLabel,
    /// Hotkey description for pausing/resuming recording.
    HotkeyPauseResumeLabel,
    /// Button label to export a recent video recording as an animated GIF.
    BtnExportGif,
    /// Badge label for animated GIF media items.
    GifBadge,
    /// Status notification when animated GIF export completes.
    StatusGifExported,
    /// Section header for Animated GIF Export settings.
    GifExportSectionTitle,
    /// Label for GIF resolution scale selector.
    GifScaleLabel,
    /// Label for GIF frame rate selector.
    GifFpsLabel,
    /// Label for GIF maximum clip duration slider.
    GifMaxDurationLabel,
}

impl I18nKey {
    /// Returns a slice containing all variants of [`I18nKey`] for automated verification.
    #[must_use]
    pub const fn all_variants() -> &'static [I18nKey] {
        &[
            I18nKey::AppTitle,
            I18nKey::AppSubtitle,
            I18nKey::StartRecording,
            I18nKey::StopRecording,
            I18nKey::PauseRecording,
            I18nKey::ResumeRecording,
            I18nKey::InstantReplaySave,
            I18nKey::SelectArea,
            I18nKey::SelectWindow,
            I18nKey::SelectMonitor,
            I18nKey::HudRecordingTime,
            I18nKey::HudAudioLevel,
            I18nKey::SettingsTitle,
            I18nKey::GeneralTab,
            I18nKey::VideoTab,
            I18nKey::AudioTab,
            I18nKey::OutputDirectory,
            I18nKey::FrameRateLabel,
            I18nKey::ResolutionLabel,
            I18nKey::HardwareAcceleration,
            I18nKey::StatusIdle,
            I18nKey::StatusRecording,
            I18nKey::StatusPaused,
            I18nKey::StatusSaving,
            I18nKey::ErrorPermissionDenied,
            I18nKey::ErrorDeviceLost,
            I18nKey::ErrorTargetNotFound,
            I18nKey::ErrorUnknown,
            I18nKey::OpenRecordingsFolder,
            I18nKey::ExitApp,
            I18nKey::TrayOpen,
            I18nKey::TrayTooltip,
            I18nKey::TrayMinimizeToTray,
            I18nKey::ProBadge,
            I18nKey::EnginePill,
            I18nKey::HwReadyPill,
            I18nKey::ModeFullScreen,
            I18nKey::ModeWindow,
            I18nKey::ModeFreeRegion,
            I18nKey::ModePresets,
            I18nKey::PrimaryDisplayLabel,
            I18nKey::CaptureBackendLabel,
            I18nKey::SystemAudioLabel,
            I18nKey::MicrophoneLabel,
            I18nKey::WebcamPipLabel,
            I18nKey::HwEncoderLabel,
            I18nKey::DestinationFolder,
            I18nKey::BtnChange,
            I18nKey::BtnOpenFolder,
            I18nKey::BtnStartRecordingF9,
            I18nKey::BtnStopRecordingF9,
            I18nKey::BtnCaptureImageF10,
            I18nKey::RecentRecording,
            I18nKey::BtnPlay,
            I18nKey::BtnShowInFolder,
            I18nKey::EngineReady,
            I18nKey::SettingsEncoderTitle,
            I18nKey::SettingsTabVideoEncoder,
            I18nKey::SettingsTabAudioChannels,
            I18nKey::SettingsTabOutputFormat,
            I18nKey::SettingsTabHotkeys,
            I18nKey::SettingsTabPerfBuffer,
            I18nKey::HwEncoderBackendSection,
            I18nKey::RecommendedBadge,
            I18nKey::DetectedGpuLabel,
            I18nKey::BitrateSectionTitle,
            I18nKey::BitrateControlMode,
            I18nKey::TargetBitrateLabel,
            I18nKey::NvencPresetLabel,
            I18nKey::ProfileLabel,
            I18nKey::KeyframesLabel,
            I18nKey::LatencyMetric,
            I18nKey::DroppedFramesMetric,
            I18nKey::VramMetric,
            I18nKey::BtnResetDefaults,
            I18nKey::BtnCancel,
            I18nKey::BtnSave,
            I18nKey::NavDashboard,
            I18nKey::NavSettings,
            I18nKey::PrimaryBadge,
            I18nKey::UiScaleLabel,
            I18nKey::UiScaleSmall,
            I18nKey::UiScaleNormal,
            I18nKey::UiScaleLarge,
            I18nKey::UiScaleExtraLarge,
            I18nKey::BtnCopyPath,
            I18nKey::BtnDelete,
            I18nKey::StatusCopiedToClipboard,
            I18nKey::StatusFileDeleted,
            I18nKey::TrayStartStopRecording,
            I18nKey::TrayTakeSnapshot,
            I18nKey::TrayOpenFolder,
            I18nKey::SystemAudioVolumeLabel,
            I18nKey::MicrophoneGainLabel,
            I18nKey::BtnMute,
            I18nKey::BtnUnmute,
            I18nKey::AudioSampleRateLabel,
            I18nKey::AudioSpecsDescription,
            I18nKey::CursorEffectsSection,
            I18nKey::CursorHaloToggle,
            I18nKey::CursorRipplesToggle,
            I18nKey::CursorHaloRadiusLabel,
            I18nKey::CursorColorLabel,
            I18nKey::CursorColorYellow,
            I18nKey::CursorColorBlue,
            I18nKey::CursorColorNeon,
            I18nKey::SnapshotBadge,
            I18nKey::BtnCloseModal,
            I18nKey::Mp4RoadmapNotice,
            I18nKey::HotkeyStartStopLabel,
            I18nKey::HotkeySnapshotLabel,
            I18nKey::HotkeyReplayLabel,
            I18nKey::HotkeyPauseResumeLabel,
            I18nKey::BtnExportGif,
            I18nKey::GifBadge,
            I18nKey::StatusGifExported,
            I18nKey::GifExportSectionTitle,
            I18nKey::GifScaleLabel,
            I18nKey::GifFpsLabel,
            I18nKey::GifMaxDurationLabel,
        ]
    }
}

/// Retrieves the localized string for a given key and target language.
///
/// Guaranteed zero-allocation, returning a `'static` string slice.
///
/// # Arguments
///
/// * `key` - The strongly typed message key.
/// * `lang` - The target language.
///
/// # Returns
///
/// The localized text representation.
///
/// # Examples
///
/// ```rust
/// use rec_core::i18n::{text, I18nKey, Language};
///
/// assert_eq!(text(I18nKey::StartRecording, Language::English), "Start Recording");
/// assert_eq!(text(I18nKey::StartRecording, Language::Spanish), "Iniciar Grabación");
/// ```
#[must_use]
pub fn text(key: I18nKey, lang: Language) -> &'static str {
    match lang {
        Language::English => match key {
            I18nKey::AppTitle => "Screen Recorder Pro",
            I18nKey::AppSubtitle => "High-Performance Cross-Platform Screen Recording",
            I18nKey::StartRecording => "Start Recording",
            I18nKey::StopRecording => "Stop Recording",
            I18nKey::PauseRecording => "Pause Recording",
            I18nKey::ResumeRecording => "Resume Recording",
            I18nKey::InstantReplaySave => "Save Instant Replay",
            I18nKey::SelectArea => "Select Area",
            I18nKey::SelectWindow => "Select Window",
            I18nKey::SelectMonitor => "Select Monitor",
            I18nKey::HudRecordingTime => "Recording Time",
            I18nKey::HudAudioLevel => "Audio Level",
            I18nKey::SettingsTitle => "Settings",
            I18nKey::GeneralTab => "General",
            I18nKey::VideoTab => "Video",
            I18nKey::AudioTab => "Audio",
            I18nKey::OutputDirectory => "Output Directory",
            I18nKey::FrameRateLabel => "Frame Rate (FPS)",
            I18nKey::ResolutionLabel => "Resolution",
            I18nKey::HardwareAcceleration => "Hardware Acceleration (GPU)",
            I18nKey::StatusIdle => "Ready",
            I18nKey::StatusRecording => "Recording...",
            I18nKey::StatusPaused => "Paused",
            I18nKey::StatusSaving => "Saving video...",
            I18nKey::ErrorPermissionDenied => "Screen recording permission was denied by the operating system.",
            I18nKey::ErrorDeviceLost => "Graphics device was lost. Please restart recording.",
            I18nKey::ErrorTargetNotFound => "Selected monitor or target window was closed or not found.",
            I18nKey::ErrorUnknown => "An unexpected recording error occurred.",
            I18nKey::OpenRecordingsFolder => "Open Recordings Folder",
            I18nKey::ExitApp => "Exit",
            I18nKey::TrayOpen => "Open Screen Recorder",
            I18nKey::TrayTooltip => "Screen Recorder Pro",
            I18nKey::TrayMinimizeToTray => "Minimize to System Tray",
            I18nKey::ProBadge => "PRO",
            I18nKey::EnginePill => "Core v2.4 (Sub-5ms)",
            I18nKey::HwReadyPill => "Hardware Ready (Sub-3ms)",
            I18nKey::ModeFullScreen => "Full Monitor",
            I18nKey::ModeWindow => "Window",
            I18nKey::ModeFreeRegion => "Custom Region",
            I18nKey::ModePresets => "Presets",
            I18nKey::PrimaryDisplayLabel => "PRIMARY DISPLAY",
            I18nKey::CaptureBackendLabel => "Capture Backend:",
            I18nKey::SystemAudioLabel => "System Audio",
            I18nKey::MicrophoneLabel => "Microphone",
            I18nKey::WebcamPipLabel => "Webcam (PIP)",
            I18nKey::HwEncoderLabel => "HW Encoder:",
            I18nKey::DestinationFolder => "Destination:",
            I18nKey::BtnChange => "Change",
            I18nKey::BtnOpenFolder => "Open Folder",
            I18nKey::BtnStartRecordingF9 => "Start Recording",
            I18nKey::BtnStopRecordingF9 => "Stop Recording",
            I18nKey::BtnCaptureImageF10 => "Take Screenshot",
            I18nKey::RecentRecording => "Recent",
            I18nKey::BtnPlay => "Play",
            I18nKey::BtnShowInFolder => "Show",
            I18nKey::EngineReady => "Capture Engine Ready",
            I18nKey::SettingsEncoderTitle => "Settings / Encoder Setup",
            I18nKey::SettingsTabVideoEncoder => "Video & Encoder",
            I18nKey::SettingsTabAudioChannels => "Audio & Channels",
            I18nKey::SettingsTabOutputFormat => "Output & Format",
            I18nKey::SettingsTabHotkeys => "Keyboard Shortcuts",
            I18nKey::SettingsTabPerfBuffer => "Performance & Buffer",
            I18nKey::HwEncoderBackendSection => "Backend & Hardware Encoder",
            I18nKey::RecommendedBadge => "RECOMMENDED",
            I18nKey::DetectedGpuLabel => "Detected GPU:",
            I18nKey::BitrateSectionTitle => "Bitrate & Quality Control",
            I18nKey::BitrateControlMode => "Rate Control Mode",
            I18nKey::TargetBitrateLabel => "Target Bitrate",
            I18nKey::NvencPresetLabel => "Encoding Preset",
            I18nKey::ProfileLabel => "Profile",
            I18nKey::KeyframesLabel => "Keyframes (GOP)",
            I18nKey::LatencyMetric => "ENGINE LATENCY:",
            I18nKey::DroppedFramesMetric => "DROPPED FRAMES:",
            I18nKey::VramMetric => "VRAM USAGE:",
            I18nKey::BtnResetDefaults => "Reset to Defaults",
            I18nKey::BtnCancel => "Cancel",
            I18nKey::BtnSave => "Save Changes",
            I18nKey::NavDashboard => "General",
            I18nKey::NavSettings => "⚙ Settings",
            I18nKey::PrimaryBadge => "★ Primary",
            I18nKey::UiScaleLabel => "UI & Font Scale",
            I18nKey::UiScaleSmall => "Compact (85%)",
            I18nKey::UiScaleNormal => "Normal (100%)",
            I18nKey::UiScaleLarge => "Large (115%)",
            I18nKey::UiScaleExtraLarge => "Extra (130%)",
            I18nKey::BtnCopyPath => "Copy Path",
            I18nKey::BtnDelete => "Delete",
            I18nKey::StatusCopiedToClipboard => "✓ Path copied to clipboard",
            I18nKey::StatusFileDeleted => "✓ File deleted successfully",
            I18nKey::TrayStartStopRecording => "Start / Stop Recording [F9]",
            I18nKey::TrayTakeSnapshot => "Take Screenshot [F10]",
            I18nKey::TrayOpenFolder => "Open Recordings Folder",
            I18nKey::SystemAudioVolumeLabel => "System Audio Volume",
            I18nKey::MicrophoneGainLabel => "Microphone Gain",
            I18nKey::BtnMute => "Mute",
            I18nKey::BtnUnmute => "Unmute",
            I18nKey::AudioSampleRateLabel => "Sample Rate & Precision",
            I18nKey::AudioSpecsDescription => "48,000 Hz • Stereo Float32 PCM • Sub-1ms Mix",
            I18nKey::CursorEffectsSection => "Smart Cursor Effects (Halo & Click Ripples)",
            I18nKey::CursorHaloToggle => "Cursor Highlight Halo",
            I18nKey::CursorRipplesToggle => "Click Ripple Waves",
            I18nKey::CursorHaloRadiusLabel => "Halo Radius",
            I18nKey::CursorColorLabel => "Halo Color",
            I18nKey::CursorColorYellow => "Golden Yellow",
            I18nKey::CursorColorBlue => "Electric Blue",
            I18nKey::CursorColorNeon => "Neon Emerald",
            I18nKey::SnapshotBadge => "Snapshot",
            I18nKey::BtnCloseModal => "✖ Close",
            I18nKey::Mp4RoadmapNotice => "ℹ Hardware-accelerated MP4 (NVENC/QSV/AMF) is scheduled for Phase 2 (v0.3.0). Recording in OpenDML AVI.",
            I18nKey::HotkeyStartStopLabel => "Start / Stop Recording:",
            I18nKey::HotkeySnapshotLabel => "Take Screenshot / Snapshot:",
            I18nKey::HotkeyReplayLabel => "Save Instant Replay:",
            I18nKey::HotkeyPauseResumeLabel => "Pause / Resume Recording:",
            I18nKey::BtnExportGif => "Export GIF",
            I18nKey::GifBadge => "Animated GIF",
            I18nKey::StatusGifExported => "✓ Animated GIF exported successfully",
            I18nKey::GifExportSectionTitle => "Animated GIF HD Export (5–15s Clips)",
            I18nKey::GifScaleLabel => "GIF Resolution Scale",
            I18nKey::GifFpsLabel => "GIF Frame Rate",
            I18nKey::GifMaxDurationLabel => "Max GIF Clip Duration",
        },
        Language::Spanish => match key {
            I18nKey::AppTitle => "Screen Recorder Pro",
            I18nKey::AppSubtitle => "Grabación de Pantalla Multiplataforma de Alto Rendimiento",
            I18nKey::StartRecording => "Iniciar Grabación",
            I18nKey::StopRecording => "Detener Grabación",
            I18nKey::PauseRecording => "Pausar Grabación",
            I18nKey::ResumeRecording => "Reanudar Grabación",
            I18nKey::InstantReplaySave => "Guardar Repetición Instantánea",
            I18nKey::SelectArea => "Seleccionar Área",
            I18nKey::SelectWindow => "Seleccionar Ventana",
            I18nKey::SelectMonitor => "Seleccionar Monitor",
            I18nKey::HudRecordingTime => "Tiempo de Grabación",
            I18nKey::HudAudioLevel => "Nivel de Audio",
            I18nKey::SettingsTitle => "Configuración",
            I18nKey::GeneralTab => "General",
            I18nKey::VideoTab => "Video",
            I18nKey::AudioTab => "Audio",
            I18nKey::OutputDirectory => "Carpeta de Destino",
            I18nKey::FrameRateLabel => "Fotogramas por Segundo (FPS)",
            I18nKey::ResolutionLabel => "Resolución",
            I18nKey::HardwareAcceleration => "Aceleración por Hardware (GPU)",
            I18nKey::StatusIdle => "Listo",
            I18nKey::StatusRecording => "Grabando...",
            I18nKey::StatusPaused => "Pausado",
            I18nKey::StatusSaving => "Guardando video...",
            I18nKey::ErrorPermissionDenied => "Permiso de grabación de pantalla denegado por el sistema operativo.",
            I18nKey::ErrorDeviceLost => "El dispositivo gráfico se desconectó o reinició. Reinicie la grabación.",
            I18nKey::ErrorTargetNotFound => "El monitor o la ventana seleccionada no fue encontrada o se cerró.",
            I18nKey::ErrorUnknown => "Ocurrió un error inesperado durante la grabación.",
            I18nKey::OpenRecordingsFolder => "Abrir Carpeta de Grabaciones",
            I18nKey::ExitApp => "Salir",
            I18nKey::TrayOpen => "Abrir Grabador de Pantalla",
            I18nKey::TrayTooltip => "Screen Recorder Pro",
            I18nKey::TrayMinimizeToTray => "Minimizar a la Bandeja",
            I18nKey::ProBadge => "PRO",
            I18nKey::EnginePill => "Núcleo v2.4 (Sub-5ms)",
            I18nKey::HwReadyPill => "Hardware Listo (Sub-3ms)",
            I18nKey::ModeFullScreen => "Monitor Completo",
            I18nKey::ModeWindow => "Ventana",
            I18nKey::ModeFreeRegion => "Región Libre",
            I18nKey::ModePresets => "Preajustes",
            I18nKey::PrimaryDisplayLabel => "PANTALLA PRIMARIA",
            I18nKey::CaptureBackendLabel => "Backend de Grabación:",
            I18nKey::SystemAudioLabel => "Audio del Sistema",
            I18nKey::MicrophoneLabel => "Micrófono",
            I18nKey::WebcamPipLabel => "Cámara Web (PIP)",
            I18nKey::HwEncoderLabel => "Encoder HW:",
            I18nKey::DestinationFolder => "Destino:",
            I18nKey::BtnChange => "Cambiar",
            I18nKey::BtnOpenFolder => "Abrir Carpeta",
            I18nKey::BtnStartRecordingF9 => "Iniciar Grabación",
            I18nKey::BtnStopRecordingF9 => "Detener Grabación",
            I18nKey::BtnCaptureImageF10 => "Captura de Imagen",
            I18nKey::RecentRecording => "Reciente",
            I18nKey::BtnPlay => "Reproducir",
            I18nKey::BtnShowInFolder => "Mostrar",
            I18nKey::EngineReady => "Motor de Captura Listo",
            I18nKey::SettingsEncoderTitle => "Configuración / Ajustes del Encoder",
            I18nKey::SettingsTabVideoEncoder => "Video y Codificador",
            I18nKey::SettingsTabAudioChannels => "Audio y Canales",
            I18nKey::SettingsTabOutputFormat => "Salida y Formato",
            I18nKey::SettingsTabHotkeys => "Atajos de Teclado",
            I18nKey::SettingsTabPerfBuffer => "Rendimiento y Búfer",
            I18nKey::HwEncoderBackendSection => "Backend y Codificador por Hardware",
            I18nKey::RecommendedBadge => "RECOMENDADO",
            I18nKey::DetectedGpuLabel => "GPU Detectada:",
            I18nKey::BitrateSectionTitle => "Control de Tasa y Calidad (Bitrate)",
            I18nKey::BitrateControlMode => "Modo de Control de Tasa",
            I18nKey::TargetBitrateLabel => "Tasa de Bits Objetivo (Target Bitrate)",
            I18nKey::NvencPresetLabel => "Preset de Codificación NVENC",
            I18nKey::ProfileLabel => "Perfil (Profile)",
            I18nKey::KeyframesLabel => "Fotogramas Clave (GOP)",
            I18nKey::LatencyMetric => "LATENCIA MOTOR:",
            I18nKey::DroppedFramesMetric => "FRAMES PERDIDOS:",
            I18nKey::VramMetric => "USO DE VRAM:",
            I18nKey::BtnResetDefaults => "Restablecer Valores por Defecto",
            I18nKey::BtnCancel => "Cancelar",
            I18nKey::BtnSave => "Guardar Cambios",
            I18nKey::NavDashboard => "Panel",
            I18nKey::NavSettings => "⚙ Ajustes",
            I18nKey::PrimaryBadge => "★ Principal",
            I18nKey::UiScaleLabel => "Escala de Fuente e Interfaz",
            I18nKey::UiScaleSmall => "Compacto (85%)",
            I18nKey::UiScaleNormal => "Estándar (100%)",
            I18nKey::UiScaleLarge => "Grande (115%)",
            I18nKey::UiScaleExtraLarge => "Extra Grande (130%)",
            I18nKey::BtnCopyPath => "Copiar Ruta",
            I18nKey::BtnDelete => "Eliminar",
            I18nKey::StatusCopiedToClipboard => "✓ Ruta copiada al portapapeles",
            I18nKey::StatusFileDeleted => "✓ Archivo eliminado correctamente",
            I18nKey::TrayStartStopRecording => "Iniciar / Detener Grabación [F9]",
            I18nKey::TrayTakeSnapshot => "Capturar Pantalla [F10]",
            I18nKey::TrayOpenFolder => "Abrir Carpeta de Grabaciones",
            I18nKey::SystemAudioVolumeLabel => "Volumen del Audio de Sistema",
            I18nKey::MicrophoneGainLabel => "Ganancia del Micrófono",
            I18nKey::BtnMute => "Silenciar",
            I18nKey::BtnUnmute => "Activar Sonido",
            I18nKey::AudioSampleRateLabel => "Frecuencia de Muestreo y Precisión",
            I18nKey::AudioSpecsDescription => "48.000 Hz • PCM Estéreo Float32 • Mezcla Sub-1ms",
            I18nKey::CursorEffectsSection => "Efectos Inteligentes de Cursor (Halo y Clics)",
            I18nKey::CursorHaloToggle => "Halo Resaltador de Cursor",
            I18nKey::CursorRipplesToggle => "Ondas de Clic (Ripples)",
            I18nKey::CursorHaloRadiusLabel => "Radio del Halo",
            I18nKey::CursorColorLabel => "Color del Halo",
            I18nKey::CursorColorYellow => "Amarillo Dorado",
            I18nKey::CursorColorBlue => "Azul Eléctrico",
            I18nKey::CursorColorNeon => "Verde Neón",
            I18nKey::SnapshotBadge => "Captura",
            I18nKey::BtnCloseModal => "✖ Cerrar",
            I18nKey::Mp4RoadmapNotice => "ℹ MP4 con aceleración hardware (NVENC/QSV/AMF) programado para Fase 2 (v0.3.0). Grabando en AVI OpenDML.",
            I18nKey::HotkeyStartStopLabel => "Iniciar / Detener Grabación:",
            I18nKey::HotkeySnapshotLabel => "Captura de Imagen / Snapshot:",
            I18nKey::HotkeyReplayLabel => "Guardar Repetición Instantánea:",
            I18nKey::HotkeyPauseResumeLabel => "Pausar / Reanudar Grabación:",
            I18nKey::BtnExportGif => "Exportar GIF",
            I18nKey::GifBadge => "GIF Animado",
            I18nKey::StatusGifExported => "✓ GIF animado exportado correctamente",
            I18nKey::GifExportSectionTitle => "Exportación a GIF Animado HD (Clips 5–15s)",
            I18nKey::GifScaleLabel => "Escala de Resolución GIF",
            I18nKey::GifFpsLabel => "Cuadros por Segundo (FPS GIF)",
            I18nKey::GifMaxDurationLabel => "Duración Máxima del Clip GIF",
        },
    }
}

/// Convenience alias for [`text`].
#[inline]
#[must_use]
pub fn t(key: I18nKey, lang: Language) -> &'static str {
    text(key, lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_non_empty_and_parity() {
        for &key in I18nKey::all_variants() {
            let en = text(key, Language::English);
            let es = text(key, Language::Spanish);

            assert!(
                !en.trim().is_empty(),
                "English translation for {:?} must not be empty",
                key
            );
            assert!(
                !es.trim().is_empty(),
                "Spanish translation for {:?} must not be empty",
                key
            );

            // Enforce RULE-I18N-001 Section 3: Spanish must not be an accidental copy-paste of English
            // unless it is an identical brand/technical identifier.
            let is_identical_brand_term = matches!(
                key,
                I18nKey::AppTitle
                    | I18nKey::TrayTooltip
                    | I18nKey::ProBadge
                    | I18nKey::GeneralTab
                    | I18nKey::VideoTab
                    | I18nKey::AudioTab
            );
            if !is_identical_brand_term {
                assert_ne!(
                    en, es,
                    "Spanish translation for {:?} is identical to English ('{}')",
                    key, en
                );
            }
        }
    }

    #[test]
    fn test_language_display_names() {
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::Spanish.code(), "es");
    }
}
