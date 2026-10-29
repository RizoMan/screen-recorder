# CORE ARCHITECTURE PRINCIPLES: SOLID, KISS, DRY, AND IDIOMATIC RUST OOP
ID: `RULE-ARCH-001`
Status: MANDATORY / CORE DESIGN DOCTRINE

---

## 1. Overview and Incongruity Resolution
While classic Object-Oriented Programming (OOP) relies heavily on class inheritance and mutable shared state, idiomatic Rust achieves clean software design through:
- **Encapsulation via visibility and typestates**: private fields, validated constructors (`new() -> Result<Self, E>`), and RAII.
- **Polymorphism via Traits**: Static dispatch (`impl Trait`) for zero-cost abstraction and dynamic dispatch (`Box<dyn Trait>`) for pluggable runtime backends.
- **Composition over Inheritance**: Struct composition, tuple structs, and delegation rather than deep class trees.

---

## 2. SOLID Principles in Rust

### S — Single Responsibility Principle (SRP)
- Each crate, module, struct, and trait must address exactly one concern.
  - `rec-core`: Pure models, timestamps, frame buffers, and domain logic only. Zero platform API dependencies.
  - `rec-platform-*`: Platform capture/audio bindings only. No GUI logic, no encoding logic.
  - `rec-engine`: Pipeline coordination and buffer routing only. No UI drawing, no Win32/X11 direct calls.
  - `rec-app`: Presentation and user interaction only.
- **Anti-Monolith Rule (Module-Level SRP)**:
  - Never combine domain/UI state models, controller event dispatch, and multiple view panels inside a single monolithic file exceeding ~500 lines.
  - In `rec-app`, separate state models (`gui/models.rs`), controller actions (`gui.rs`), dashboard views (`gui/dashboard.rs`), modals (`gui/trim_modal.rs`), and settings (`gui/settings.rs`) into dedicated submodules.
  - Never embed inline `#[cfg(test)] mod tests { ... }` blocks inside production `src/*.rs` files; keep all test suites in dedicated `tests/` files so production modules remain concise and single-purpose.

### O — Open/Closed Principle (OCP)
- Systems must be open for extension, but closed for modification.
- Adding a new capture backend (e.g., macOS ScreenCaptureKit or Linux PipeWire) requires adding a new struct implementing `CaptureBackend` in its respective platform crate. It requires **zero changes** to `rec-engine`'s recording pipeline.

### L — Liskov Substitution Principle (LSP)
- Implementations of traits (`CaptureBackend`, `AudioBackend`, `EncoderBackend`) must be interchangeable without violating callers' expectations or invariants.
- Any backend must yield frames that conform to `FrameDescriptor`, valid monotonically increasing `pts`, and identical typed error guarantees.

### I — Interface Segregation Principle (ISP)
- Traits must be small, cohesive, and focused.
- Never force a backend to implement unrelated capabilities:
  - `CaptureBackend` only captures video frames.
  - `AudioBackend` only handles PCM audio streams.
  - `EncoderBackend` only compresses frames.
  - `MuxerBackend` only multiplexes packets into containers.

### D — Dependency Inversion Principle (DIP)
- High-level modules must not depend on low-level modules; both must depend on abstractions.
- `rec-engine` defines the traits (`CaptureBackend`, `AudioBackend`, etc.) or depends on traits from `rec-core`.
- Concrete platform implementations (`rec-platform-windows`, `rec-platform-linux`, `rec-platform-macos`) implement these abstractions.
- `rec-app` acts as the composition root, injecting the platform backend into `rec-engine` at startup.

---

## 3. KISS (Keep It Simple, Stupid)
- Avoid premature micro-optimizations, excessive macro DSLs, or complex type-level programming where standard structs, enums, and channels suffice.
- Use simple, predictable lock-free ring buffers (SPSC) and dedicated OS threads for latency-critical capture, avoiding overhead or unpredictable scheduling from heavy async runtimes in the hot frame path.
- **Zero Stringly-Typed Actions or Magic String Comparisons**:
  - Never return raw string slices (`Option<&'static str>`) from UI widgets to represent actions or compare display strings (`duration_str == "Snapshot"`) to infer domain state. Always use explicit, strongly-typed Rust `enum`s (`HudAction`, `RecentMediaKind`).

---

## 4. DRY (Don't Repeat Yourself)
- Frame timestamp calculations (PTS, DTS, 90kHz or nanosecond time bases), pixel format byte sizing, audio resampler formulas, and i18n catalogs must exist in exactly one place: `rec-core`.
- Reusable platform helpers (e.g., COM initialization guards, WinRT string conversions) must be written once in utility modules.
- **Single Source of Truth in State & Controller Actions**:
  - Never duplicate state flags across `RecorderGuiApp` and `AppConfig` (e.g., use `config.microphone_muted` everywhere).
  - Never copy-paste multi-line action blocks across UI buttons, hotkeys (`F9`/`F10`), and system tray handlers; centralize them in controller methods (`toggle_recording`, `start_recording`, `stop_recording`, `trigger_screenshot`).
  - Reusable UI widgets (such as VU meters, pill badges, and card frames) must live in `theme.rs` and be shared across the HUD, Dashboard, and Settings views.

---

## 5. POO / Idiomatic Rust Object-Oriented Patterns
- **Constructor Invariants**: Always provide constructors like `Self::new(...) -> Result<Self, Error>` that prevent constructing objects in invalid states.
- **Strict Encapsulation**: Struct fields are private (`pub(crate)` or private to module). External mutation occurs through explicit, validated methods.
- **Strategy & State Patterns**: Represent states as Rust enums or typestates (e.g., `RecorderState::Idle`, `RecorderState::Recording { started_at }`, `RecorderState::Paused`) to enforce valid state transitions at compile time or via robust state machines.
- **RAII Lifecycle**: Every resource-holding object cleans itself up upon leaving scope via `Drop`.
