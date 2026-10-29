# SCREEN RECORDER — CONSTITUCIÓN ARQUITECTURAL MAESTRA (AGENTS.MD)
Document ID: `CONST-ARCH-001`
Status: VIGENTE / CONSTITUCIÓN SUPREMA
Versión: 0.1.0

---

## 1. MISIÓN Y PROPÓSITO
Desarrollar un grabador de pantalla multiplataforma (Windows, Linux, macOS) de grado comercial, ultra-alto rendimiento (60–120 FPS, latencia <16ms por frame, 0 fugas de memoria) y 100% memoria segura en Rust idiomático.

El proyecto se rige de manera inviolable por **SPEC-DRIVEN DEVELOPMENT (SDD)**, **Clean Architecture** mediante Inversión de Dependencias (DIP), y principios **SOLID, KISS, DRY y POO Idiomática en Rust**.

---

## 2. ANÁLISIS DE INCONGRUENCIAS Y RESOLUCIÓN ARQUITECTÓNICA

| Incongruencia Aparente | Conflicto Técnico | Resolución Arquitectural Adoptada |
| :--- | :--- | :--- |
| **"100% safe Rust" vs. APIs Nativas de SO** | DirectX, WinRT, PipeWire y ScreenCaptureKit requieren llamadas FFI a C/C++ que el compilador de Rust clasifica como `unsafe`. | **Frontera de Seguridad Estricta (Boundary Isolation)**: `rec-core` y `rec-engine` tienen `#![forbid(unsafe_code)]`. Los crates de plataforma (`rec-platform-*`) confinan el FFI en módulos adaptadores internos privados, documentando cada bloque con `// SAFETY: <justificación>`, y exponen al resto del sistema interfaces 100% seguras basadas en Traits y RAII. |
| **"POO" (OOP) vs. Rust Idiomático** | Rust no posee herencia de clases ni polimorfismo de subtipos mediante jerarquías de clases. Intentar forzar OOP clásico genera antipatrones de `Rc<RefCell<...>>`. | **POO Idiomática en Rust**: Reemplazar herencia por **Composición** (`Composition over Inheritance`). Polimorfismo mediante **Traits** (despacho dinámico `Box<dyn CaptureBackend>` para desacoplar el motor). Encapsulación estricta con constructores validados (`new() -> Result<Self, E>`) e invariantes tipadas. |
| **Compilación Multiplataforma en Host Windows** | Crates para Linux (`pipewire`) o macOS (`screencapturekit`) fallarían en Windows si se vinculan directamente con librerías nativas del sistema foráneo. | **Condicionalidad Estricta y Stubs de Desarrollo**: Los crates `rec-platform-linux` y `rec-platform-macos` implementan los mismos traits de `rec-engine`, usando `#[cfg(target_os = ...)]` para las llamadas al SO y stubs con errores tipados `UnsupportedPlatform` en plataformas anfitrionas distintas. Así, `cargo check --workspace` y `cargo test --workspace` compilan con 0 errores en cualquier plataforma. |
| **Bajo Consumo vs. Bucle de Captura a 60 FPS** | La asignación dinámica de memoria en cada frame genera latencia de recolección y jitter visual. | **Cero Copias y Ring Buffers Lock-Free**: Canales Single-Producer Single-Consumer (SPSC) lock-free preasignados con memoria contigua en `rec-core` y traspaso de buffers por movimiento de punteros (`D3D11 Texture / Hardware Buffers`). |

---

## 3. TOPOLOGÍA DEL CARGO WORKSPACE (CLEAN ARCHITECTURE / DIP)

```
                            ┌───────────────────────────────────┐
                            │             rec-app               │
                            │  (GUI eframe/egui, CLI, Tray, HUD)│
                            └─────────────────┬─────────────────┘
                                              │ Inyecta backend
                                              ▼
                            ┌───────────────────────────────────┐
                            │            rec-engine             │
                            │  (Pipeline Orchestrator, Muxer,   │
                            │   Instant Replay, CaptureBackend) │
                            └─────────┬───────────────┬─────────┘
             Implementa Backend       │               │ Depende de dominio
      ┌───────────────────────────────┤               ▼
      │                               │     ┌───────────────────┐
      ▼                               ▼     │     rec-core      │
┌──────────────────────┐ ┌──────────────────────┐   │ (Frames, PTS/DTS, │
│ rec-platform-windows │ │ rec-platform-linux   │   │  Audio, RingBuffer│
│ (WGC, DXGI, WASAPI)  │ │ (PipeWire, Portal)   │   │  Config, i18n)    │
└──────────────────────┘ └──────────────────────┘   └───────────────────┘
      ▲
      │ Implementa Backend
┌──────────────────────┐
│ rec-platform-macos   │
│ (ScreenCaptureKit)   │
└──────────────────────┘
```

### Detalle de Responsabilidades por Crate

1. **`crates/rec-core`** (Pure Domain & Foundation):
   - Modelos de frames (`FrameDescriptor`, `PixelFormat::Rgba`, `PixelFormat::Nv12`, `PixelFormat::I420`).
   - Sincronización A/V (Presentation Timestamps `Pts`, Decode Timestamps `Dts`).
   - Mezclador de audio (PCM `f32`, remuestreo a 48 kHz estéreo).
   - Ring buffers lock-free para cero copias de frames.
   - Abstracciones de multiplexado y contenedores (MP4, MKV).
   - Configuración atómica persistente y sanitizada (`AppConfig`).
   - Módulo de internacionalización con paridad 100% bilingüe (EN/ES) en tiempo cero (`rec_core::i18n`).
   - **Regla estricta**: Cero dependencias con APIs de SO o llamadas al kernel (`#![forbid(unsafe_code)]`).

2. **`crates/rec-platform-windows`**:
   - Captura por Windows Graphics Capture (WGC / WinRT) para ventanas individuales y monitores.
   - Captura por DXGI Desktop Duplication API (D3D11 textures) para captura de escritorio a 60–120 FPS ultra-baja latencia.
   - Audio Loopback mediante WASAPI (audio de sistema + micrófono sincronizado).
   - Codificación por hardware vía Media Foundation (NVENC, Intel QSV, AMD AMF) con fallback software.
   - Hotkeys globales de Win32 y bandeja de sistema nativa.

3. **`crates/rec-platform-linux`**:
   - Captura Wayland moderna vía PipeWire y `xdg-desktop-portal` (Screencast Portal).
   - Captura de fallback para X11 mediante XSHM / XDamage / XComposite.
   - Loopback y micrófono mediante PipeWire / PulseAudio.
   - Aceleración por hardware vía VA-API.

4. **`crates/rec-platform-macos`**:
   - Captura nativa moderna mediante ScreenCaptureKit (macOS 12.3+) con exclusión de ventana del propio grabador.
   - Codificación de hardware vía VideoToolbox (H.264 / HEVC).
   - Audio de sistema y micrófono mediante CoreAudio HAL.

5. **`crates/rec-engine`** (Pipeline Orchestrator & DIP):
   - Define los traits unificados:
     * `CaptureBackend`: `fn start_capture(&mut self, target: CaptureTarget, frame_tx: Sender<VideoFrame>) -> Result<(), CaptureError>;`
     * `AudioBackend`: `fn start_audio_capture(&mut self, audio_tx: Sender<AudioFrame>) -> Result<(), AudioError>;`
     * `EncoderBackend`: `fn encode_frame(&mut self, frame: VideoFrame) -> Result<(), EncoderError>;`
     * `MuxerBackend`: multiplexado en disco con sincronización PTS.
   - Orquestador de pipeline multihilo: Hilo de Captura -> Ring Buffer Lock-free -> Hilo de Codificación -> Hilo Muxer.
   - Control de pausa, reanudación y buffer circular de repetición instantánea (*Instant Replay* de los últimos N segundos en RAM).

6. **`crates/rec-app`** (GUI & CLI Presentation):
   - GUI nativa en `eframe`/`egui` con modo oscuro premium, tipografía vectorial y micro-animaciones.
   - Selector de área interactivo con overlay translúcido.
   - HUD flotante de grabación compacto y no intrusivo con temporizador, vúmetro de audio (VU meter) y controles rápidos.
   - Bandeja de sistema con menú contextual.
   - Modo CLI para grabación desatendida o por comandos (`--record`, `--monitor 0`, `--area 0,0,1920,1080`, `--output out.mp4`).

---

## 4. PRINCIPIOS DE DISEÑO: SOLID, KISS, DRY Y POO

### Principios SOLID
- **S (Single Responsibility)**: Cada crate, módulo y archivo tiene un único motivo de cambio. `rec-core` define los modelos matemáticos y tipos puros; las plataformas solo interactúan con el SO; `rec-engine` solo orquesta el flujo de frames; `rec-app` separa estrictamente modelos (`gui/models.rs`), controlador (`gui.rs`), vistas de dashboard (`gui/dashboard.rs`) y paneles de ajustes (`gui/settings.rs`). **Prohibidos los archivos monolíticos (>500 líneas) que mezclen modelos, controlador y vistas**.
- **O (Open/Closed)**: El motor está abierto a nuevos backends (ej. WebRTC, cámara virtual) simplemente implementando `CaptureBackend`, sin modificar una sola línea del pipeline de `rec-engine`.
- **L (Liskov Substitution)**: Cualquier backend de captura (`WgcBackend`, `DxgiBackend`, `PipeWireBackend`, `ScreenCaptureKitBackend`) puede sustituir a otro en `rec-engine` manteniendo las garantías de entrega de `VideoFrame` y códigos de error tipados.
- **I (Interface Segregation)**: Interfaces pequeñas y atómicas. No existe una mega-interfaz monolítica: el audio tiene `AudioBackend`, el video `CaptureBackend`, el codificador `EncoderBackend`.
- **D (Dependency Inversion)**: `rec-engine` no depende de `rec-platform-windows`. Depende del trait `CaptureBackend`. La aplicación (`rec-app`) es quien compone e inyecta la implementación concreta en tiempo de ejecución.

### Principios KISS y DRY
- **KISS**: Evitar macros complejas o runtimes async innecesarios en el hot path. Canales preasignados con hilos nativos de SO proporcionan determinismo absoluto y latencia predecible. **Prohibido usar acciones tipadas con strings (`Option<&'static str>`) o comparaciones de cadenas mágicas (`duration_str == "Snapshot"`) para inferir estado; usar siempre `enum`s explícitos (`HudAction`, `RecentMediaKind`)**.
- **DRY**: Los cálculos de timestamps, conversiones de formato de píxeles, funciones de resampling de audio y catálogo de textos bilingües están centralizados en `rec-core`. **Prohibido duplicar campos de estado entre `RecorderGuiApp` y `AppConfig`, duplicar lógica de control entre botones UI/Hotkeys/Tray, o duplicar widgets visuales (como vúmetros o badges) fuera de `theme.rs`**.

### POO Idiomática en Rust
- Constructores `new() -> Result<Self, Error>` que hacen imposible crear instancias en estado inválido.
- Encapsulación total: campos privados, estado inmutable por defecto, y mutabilidad controlada por métodos.
- Máquinas de estado explícitas mediante Enums (`RecordingState::Idle`, `RecordingState::Running { start_time }`, `RecordingState::Paused { elapsed }`).
- RAII implacable: todo recurso que requiera liberación implementa `Drop`.

---

## 5. REGLAS INNEGOCIABLES (IRONCLAD RULES — `.agents/rules/`)

| ID de Regla | Archivo Normativo | Mandato Inviolable |
| :--- | :--- | :--- |
| **`RULE-SDD-001`** | `.agents/rules/sdd_protocol.md` | **Spec-Driven Development**: Prohibido escribir código sin antes crear/actualizar su `.spec.md` en `specs/` con `[SPEC-*]`, `[REQ-*]`, `[CONTRACT-*]`, `[NFR-*]` y `[TEST-*]`. |
| **`RULE-ARCH-001`** | `.agents/rules/architecture_principles.md` | **SOLID, KISS, DRY y POO Idiomática**: Separación estricta SRP (cero monolitos), cero duplicación de lógica/estado (DRY), cero acciones basadas en strings mágicos (KISS). |
| **`RULE-SAFE-001`** | `.agents/rules/testing_and_safety.md` | **Zero `unwrap()`/`expect()` & RAII**: Errores con `thiserror` y `?`. `#![forbid(unsafe_code)]` en `rec-core`/`rec-engine`. Todo `unsafe` en plataformas con `// SAFETY:`. |
| **`RULE-DOC-001`** | `.agents/rules/rustdoc_markdown.md` | **Rustdoc & GFM Markdown (0 Advertencias)**: Toda función pública debe incluir `# Arguments` (si recibe parámetros), `# Returns` (si retorna `!= ()`), `# Errors` (si retorna `Result`) y `# Examples` compilables. |
| **`RULE-I18N-001`** | `.agents/rules/mandatory_i18n.md` | **Internacionalización 100% Bilingüe (EN/ES)**: Cero cadenas quemadas en UI/CLI, cero `if lang == ...` ad-hoc en vistas, y paridad `en != es` verificada por test. |
| **`RULE-PKG-001`** | `.agents/rules/packaging_mandatory_testing.md` | **Empaquetado Automatizado con Bump de Versión**: Ejecutar `cargo test`, `cargo clippy -- -D warnings`, `cargo doc --no-deps` y generar instalador Inno Setup con incremento semántico en cada entrega. |

---

## 6. ÍNDICE MAESTRO DE ESPECIFICACIONES (`specs/`)

| Código | Ruta de Especificación | Título y Propósito |
| :--- | :--- | :--- |
| `[SPEC-ARCH-001]` | `specs/00_architecture/system_architecture.spec.md` | Topología del workspace, flujo de datos multihilo y DIP |
| `[SPEC-ERR-001]` | `specs/00_architecture/error_handling.spec.md` | Taxonomía de errores unificados y jerarquía `thiserror` |
| `[SPEC-CORE-001]` | `specs/01_core/capture_domain_models.spec.md` | Formatos de frame, timestamps PTS/DTS, mezclador y ring buffer |
| `[SPEC-I18N-001]` | `specs/01_core/i18n_localization.spec.md` | Catálogo de internacionalización bilingüe (EN/ES) |
| `[SPEC-PLAT-WIN-001]` | `specs/02_platform_windows/wgc_dxgi_capture.spec.md` | Captura WGC, DXGI Desktop Duplication y WASAPI loopback |
| `[SPEC-PLAT-LIN-001]` | `specs/02_platform_linux/pipewire_wayland_capture.spec.md` | Captura Wayland PipeWire Portal y X11 fallback |
| `[SPEC-PLAT-MAC-001]` | `specs/02_platform_macos/screencapturekit_capture.spec.md` | Captura ScreenCaptureKit y VideoToolbox en macOS |
| `[SPEC-GUI-001]` | `specs/03_app_ui/recorder_gui_hud.spec.md` | GUI moderna, selector de área interactivo y HUD transparente |
| `[SPEC-FEAT-REPLAY-001]` | `specs/04_features/01_instant_replay_buffer.spec.md` | Buffer circular de repetición instantánea en memoria |
| `[SPEC-FEAT-CURSOR-001]` | `specs/04_features/02_smart_cursor_effects.spec.md` | Efectos inteligentes de cursor (Halo y ondas de clic) |
| `[SPEC-FEAT-GIF-001]` | `specs/04_features/03_animated_gif_export.spec.md` | Exportación rápida de clips a formato GIF animado HD |
| `[SPEC-FEAT-TRIM-001]` | `specs/04_features/04_quick_trim_editor.spec.md` | Recorte rápido sin recodificación (Quick Trim In/Out) |
| `[SPEC-QA-001]` | `specs/05_qa/manual_qa_playbook.spec.md` | Matriz de pruebas manuales, estabilidad y verificación de audio/video |
