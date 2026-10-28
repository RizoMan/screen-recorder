# Screen Recorder Pro

A high-performance, commercial-grade, cross-platform (Windows, Linux, macOS) Screen Recorder written in 100% safe, idiomatic Rust.

Built under **Spec-Driven Development (SDD)**, Clean Architecture (DIP), and strict SOLID / KISS / DRY / OOP principles.

---

## Architecture Overview

```
                            ┌───────────────────────────────────┐
                            │             rec-app               │
                            │  (GUI eframe/egui, CLI, Tray, HUD)│
                            └─────────────────┬─────────────────┘
                                              │ Injects backend
                                              ▼
                            ┌───────────────────────────────────┐
                            │            rec-engine             │
                            │  (Pipeline Orchestrator, Muxer,   │
                            │   Instant Replay, CaptureBackend) │
                            └─────────┬───────────────┬─────────┘
             Implements Backend       │               │ Domain models
      ┌───────────────────────────────┤               ▼
      │                               │     ┌───────────────────┐
      ▼                               ▼     │     rec-core      │
┌──────────────────────┐ ┌──────────────────────┐   │ (Frames, PTS/DTS, │
│ rec-platform-windows │ │ rec-platform-linux   │   │  Audio, RingBuffer│
│ (WGC, DXGI, WASAPI)  │ │ (PipeWire, Portal)   │   │  Config, i18n)    │
└──────────────────────┘ └──────────────────────┘   └───────────────────┘
      ▲
      │ Implements Backend
┌──────────────────────┐
│ rec-platform-macos   │
│ (ScreenCaptureKit)   │
└──────────────────────┘
```

---

## Features
- **Ultra-Low Latency Desktop Capture**: 60–120 FPS using DXGI Desktop Duplication on Windows, PipeWire on Wayland, and ScreenCaptureKit on macOS.
- **Dependency Inversion Principle (DIP)**: Pure trait abstractions decoupled from operating system APIs.
- **Zero-Allocation Bilingual i18n**: 100% immediate parity in English and Spanish without burned strings.
- **Floating HUD**: Compact non-intrusive recording widget with live dual-channel audio VU meter and instant replay trigger.
- **Instant Replay Buffer**: Retroactive recording clip export from a circular in-memory buffer.
- **Ironclad Safety**: Zero `unwrap()` or `expect()` in production code. Pure domain crates strictly `#![forbid(unsafe_code)]`.

---

## Building and Testing

```bash
# Check the entire workspace
cargo check --workspace

# Run automated tests (including i18n parity and domain models)
cargo test --workspace

# Generate documentation (0 warnings guaranteed)
cargo doc --workspace --no-deps

# Run the desktop app
cargo run -p rec-app

# Run CLI mode
cargo run -p rec-app -- --record --duration 5
```

---

## Packaging

Automated packaging scripts reside in `scripts/` and `packaging/`.
- Windows: Inno Setup (`packaging/windows/installer.iss`)
- Version Bump: `powershell scripts/bump_version.ps1 -Bump patch`
