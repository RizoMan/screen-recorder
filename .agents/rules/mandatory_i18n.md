# MANDATORY BILINGUAL INTERNATIONALIZATION (i18n)
ID: `RULE-I18N-001`
Status: MANDATORY / ZERO HARDCODED STRINGS

---

## 1. Zero Burned/Hardcoded Strings Policy
- **Rule**: No user-facing string (button label, HUD text, CLI help/error, tooltip, modal dialog, system notification) may be hardcoded as a raw string literal inside UI or CLI crates (`rec-app`, `rec-engine`).
- **Enforcement**:
  - Every user-facing message must be mapped to a strongly typed enum or key in `rec_core::i18n::I18nKey`.
  - Translations are accessed exclusively via `i18n::text(key, lang)` or convenience localization helpers.
  - Ad-hoc checks such as `if lang == "es" { ... } else { ... }` in presentation code are **STRICTLY PROHIBITED**.

---

## 2. 100% Bilingual Parity (English & Spanish)
- Every single key added to `I18nKey` must have an immediate, high-fidelity translation in both English (`Language::English`) and Spanish (`Language::Spanish`).
- Leaving a placeholder like `"TODO"` or leaving one language empty is a test failure.

---

## 3. Automated Non-Emptiness & Parity Tests
- A dedicated test suite in `rec-core` (`tests/i18n_parity.rs` or unit test module) must iterate over every variant of `I18nKey`:
  1. Verify `text(key, Language::English)` is not empty.
  2. Verify `text(key, Language::Spanish)` is not empty.
  3. Verify that the Spanish translation is not an accidental copy-paste of the English string (unless it is an identical technical term like "FPS" or "ID").
