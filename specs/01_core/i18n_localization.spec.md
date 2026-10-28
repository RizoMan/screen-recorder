# SPECIFICATION: BILINGUAL INTERNATIONALIZATION (i18n)
Spec ID: `[SPEC-I18N-001]`
Status: APPROVED
Domain: Core Domain & Localization

---

## 1. Overview
Defines zero-runtime-overhead bilingual localization for English and Spanish, ensuring 100% parity across all UI, CLI, and notification strings.

---

## 2. Requirements `[REQ-I18N-001]`
1. Provide a strongly-typed `Language` enum (`English`, `Spanish`).
2. Provide a strongly-typed `I18nKey` enum covering all UI controls, HUD labels, CLI arguments, and error notifications.
3. Every `I18nKey` must have a defined, non-empty translation in both English and Spanish.
4. Support locale detection based on the operating system, with fallback to English.

---

## 3. Rust Contracts `[CONTRACT-I18N-001]`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    English,
    Spanish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum I18nKey {
    AppTitle,
    StartRecording,
    StopRecording,
    PauseRecording,
    ResumeRecording,
    InstantReplaySave,
    SelectArea,
    SelectWindow,
    SelectMonitor,
    HudRecordingTime,
    HudAudioLevel,
    SettingsTitle,
    GeneralTab,
    VideoTab,
    AudioTab,
    OutputDirectory,
    FrameRateLabel,
    ResolutionLabel,
    HardwareAcceleration,
    ErrorPermissionDenied,
    ErrorDeviceLost,
    ErrorTargetNotFound,
}

pub fn text(key: I18nKey, lang: Language) -> &'static str;
```

---

## 4. Non-Functional Requirements `[NFR-I18N-001]`
1. String lookup must be zero-allocation (`&'static str`).
2. Exact parity: no key may be left untranslated in either language.

---

## 5. Verification Criteria `[TEST-I18N-001]`
1. Automated unit test iterating through every `I18nKey` ensuring non-empty string in English and Spanish.
2. Parity check ensuring translation completeness.
