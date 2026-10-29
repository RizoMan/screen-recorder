# Screen Recorder Pro — Master Roadmap de Innovación, Features de Alto Valor y Comercialización
Document ID: `ROADMAP-REC-002`
Status: VIGENTE / HOJA DE RUTA MAESTRA DE ALTO VALOR
Versión del Documento: 0.2.2
Fecha de Actualización: 2026-10-08

---

## 1. Visión Estratégica: Filosofía "De Menos a Más" y Cierre de Brechas

Para alcanzar el máximo valor de producto comercial con calidad de grado profesional, el desarrollo adopta la directriz estricta:
**"Ir de menos a más técnicamente hablando, rellenando primero los huecos en lo que ya tenemos antes de abordar subsistemas masivos"**.

### Estado Actual del Software (v0.1.20):
- **100% Memoria Segura en Rust Idiomático** con Inversión de Dependencias (DIP) y `#![forbid(unsafe_code)]` en el núcleo (`rec-core`, `rec-engine`).
- **Búfer de Repetición Comprimido con Techo Determinista de RAM (256 MB)** y canales acotados con contrapresión estricta en GPU/CPU (cero explosiones de RAM).
- **Efectos Inteligentes de Cursor en Tiempo Real (`[SPEC-FEAT-CURSOR-001]`)**: Halo resaltador translúcido con radio ajustable (`12–64 px`), 3 paletas de color (`Amarillo Dorado`, `Azul Eléctrico`, `Verde Neón`) y animación de ondas expansivas en clics izquierdo (`#3B82F6`) y derecho (`#F59E0B`).
- **Sincronización A/V de Precisión Sub-Frame (Cero Desfase, Cero Adelanto de Audio)**: Origen unificado de reloj monotónico (`session_epoch: Instant`) compartido entre backends de captura de video (WGC/DXGI) y audio (WASAPI loopback). Mapeo estricto de timestamps referenciado al primer cuadro de video real (T0), puenteo automático de silencios para retardos de inicialización del dispositivo de audio y descarte de muestras de pre-roll. Eliminación definitiva de cualquier adelanto o atraso de audio en VLC, WMP y editores de video.
- **Indexación Jerárquica Completa OpenDML 2.0 (`indx` Super Index + `ix00`/`ix01` Standard Sub-Indices)**: Generación matemática de tablas de super-índice en cabeceras `strl` y sub-índices con `qwBaseOffset` de 64 bits en cada fragmento RIFF (AVI y AVIX). Erradicación definitiva y verificada con pruebas automáticas contra el reproductor VLC del aviso "Índice roto o perdido", permitiendo seeking instantáneo sin demoras ni advertencias en archivos de cualquier tamaño (5 GB, 20 GB, 100 GB).
- **Compatibilidad Dual AVI 1.0 / OpenDML**: Inclusión paralela de tabla clásica `idx1` en el bloque primario para reproductores legados (Windows Media Player) y super-índices para suites modernas (VLC, Premiere, DaVinci, MPC-HC).
- **Compresión JPEG Calibrada para Texto y Escritorio**: Calidad dinámica ajustada para máxima nitidez en tipografías y líneas finas con reducción drástica de tasa de bits.
- **Pacer de Video CFR Libre de Carreras**: Prevención de duplicados prematuros en codificación paralela, asegurando correspondencia exacta de cuadros y duración idéntica de audio y video al milisegundo.
- **Captura de Pantalla Instantánea (Snapshot PNG)**: Soporte completo en resoluciones multipantalla y escritorios virtuales con atajo global `F10`.
- **Control de Audio y Mezclador Dinámico en Tiempo Real**: Sliders de volumen de sistema (0-150%) y ganancia de micrófono (0-200%) con presets instantáneos, botón Mute y vúmetro estéreo en vivo.
- **Atajos Globales de Windows (Win32 Hotkeys)**: Control total mediante `F9` y `F10` en segundo plano con icono y menú contextual en la bandeja del sistema (System Tray).
- **Interfaz Obsidian Dark**: Tema oscuro premium con selector de contenedor claro (indicando MP4 en Fase 2), ajuste en vivo del zoom de fuentes y tarjeta de medios recientes bilingüe (EN/ES).
- **Empaquetado Nativo**: Instalador Inno Setup automatizado (`ScreenRecorderPro_Setup_v0.1.18.exe`).

---

## 2. Matriz de Progresión Técnica Incremental ("De Menos a Más")

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│ NIVEL 1: CERRAR HUECOS EN LO YA EXISTENTE (Completar Funcionalidades Básicas)    │
│ • Snapshot instantáneo PNG y atajo F10 [COMPLETADO v0.1.12]                      │
│ • Atajos globales de sistema (RegisterHotKey Win32 en segundo plano) [COMPLETADO v0.1.13]│
│ • Menú contextual de Bandeja de Sistema (Tray Context Menu) [COMPLETADO v0.1.13] │
│ • Acciones en medios recientes (Reproducir, Copiar ruta, Eliminar) [COMPLETADO v0.1.13] │
│ • Controles de volumen y ganancia Mic/Sistema en vivo [COMPLETADO v0.1.14]       │
└────────────────────────────────────────┬─────────────────────────────────────────┘
                                         ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│ NIVEL 2: QUICK WINS DE ALTO VALOR VISUAL (Baja Complejidad, Gran Salto de UX)   │
│ • Halo visual de cursor y animación de ondas expansivas en clics [COMPLETADO v0.1.20]│
│ • Exportación rápida de clips a formato GIF animado HD (5–15s) [COMPLETADO v0.1.23]  │
│ • Herramienta "Quick Trim" (recorte sin recodificación de inicio y fin de video) │
│ • Visualizador de teclas presionadas en vivo (Keycaster flotante)                │
└────────────────────────────────────────┬─────────────────────────────────────────┘
                                         ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│ NIVEL 3: FUNCIONALIDADES PRO PARA CREADORES (Diferenciadores Comerciales)        │
│ • Cámara Web Picture-in-Picture flotante (Webcam PiP circular / rectangular)     │
│ • Herramienta de anotaciones y dibujo sobre la pantalla en vivo (Live Draw)      │
│ • Filtros DSP de Audio: Puerta de Ruido (Noise Gate) y Normalización de Voz      │
└────────────────────────────────────────┬─────────────────────────────────────────┘
                                         ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│ NIVEL 4: MOTOR TURBO Y ARQUITECTURA AVANZADA (Máxima Escala)                     │
│ • Codificación por Hardware NVENC / QuickSync / AMF a contenedor MP4             │
│ • Smooth Auto-Zoom cinemático siguiendo la acción del mouse (Estilo Screen Studio)│
│ • Backends de captura nativos para Linux (Wayland PipeWire) y macOS (ScreenCaptureKit)
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Catálogo Detallado de Funcionalidades de Alto Valor

### BLOQUE A: Funcionalidades Estrella para Creadores y Demos (Game Changers)

#### 1. Cámara Web Picture-in-Picture Flotante (Webcam PiP Overlay)
- **Propuesta de Valor:** Indispensable para tutoriales, cursos online, presentaciones ejecutivas y soporte técnico.
- **Detalle Técnico:**
  - Captura directa del dispositivo de video mediante Media Foundation (`IMFSourceReader`) o DirectShow.
  - Renderizado en tiempo real como overlay superpuesto sobre el video con recorte circular, cuadrado o esquinas redondeadas.
  - Controles en pantalla: cambio de tamaño (pequeño, mediano, grande), reposicionamiento en las 4 esquinas con 1 clic y opción de espejo (mirror flip).
  - Efecto de sombra suave y borde personalizable en color de acento (`#3B82F6` o personalizado).
- **Crates involucrados:** `rec-platform-windows`, `rec-engine`, `rec-app`.

#### 2. Efectos Inteligentes de Cursor (Smart Cursor Halo & Click Ripples)
- **Propuesta de Valor:** Hace que el espectador nunca pierda de vista la acción en pantallas de alta resolución (4K/1440p) sin requerir edición posterior.
- **Detalle Técnico:**
  - Halo translúcido personalizable alrededor del cursor (amarillo suave, azul eléctrico o neón).
  - Animación de ondas expansivas al hacer clic: onda azul para clic izquierdo, onda naranja para clic derecho.
  - Opción de ocultar automáticamente el cursor cuando permanece estático durante más de 2 segundos.
- **Crates involucrados:** `rec-platform-windows`, `rec-core`, `rec-engine`.

#### 3. Herramienta de Anotaciones en Pantalla en Vivo (Live Draw & Annotations)
- **Propuesta de Valor:** Permite resaltar, explicar y guiar la atención directamente sobre la pantalla durante la grabación.
- **Detalle Técnico:**
  - Overlay transparente interactivo activable mediante atajo global (ej. `Ctrl + Shift + D` o tecla `F7`).
  - Herramientas:
    * **Lápiz libre / Marcador fluorescente** translúcido.
    * **Flechas rectas inteligentes** y líneas directrices.
    * **Rectángulos y elipses** con borde fino o relleno suave.
    * **Paso numérico secuencial** (círculos con 1, 2, 3... que se incrementan automáticamente con cada clic).
    * **Puntero láser temporal** que deja un rastro brillante que desaparece tras 1.5 segundos.
- **Crates involucrados:** `rec-app` (egui overlay window), `rec-engine`.

#### 4. Visualizador de Teclas en Pantalla (Keystroke Cast / Keycast HUD)
- **Propuesta de Valor:** Muy solicitado por programadores, streamers y creadores de tutoriales de software (Blender, Photoshop, VS Code).
- **Detalle Técnico:**
  - Captura global de combinaciones de teclas con modificadores (`Ctrl + C`, `Alt + Tab`, `Shift + F10`) mediante Win32 Low-Level Keyboard Hook (`WH_KEYBOARD_LL`).
  - Presentación como pills flotantes animadas con estilo Obsidian en la parte inferior central o lateral.
  - Filtro de privacidad automático: suprime la visualización cuando se detectan campos de contraseña.
- **Crates involucrados:** `rec-platform-windows`, `rec-app`.

---

### BLOQUE B: Productividad y Flujo de Trabajo Rápido (Quick Wins)

#### 5. Editor Rápido de Corte ("Quick Trim" Post-Grabación)
- **Propuesta de Valor:** El 90% de los usuarios solo necesita recortar los primeros 3 segundos de preparación y los últimos 2 segundos de cierre antes de compartir el video.
- **Detalle Técnico:**
  - Al detener la grabación, se abre un diálogo modal compacto de vista previa.
  - Barra de tiempo interactiva con deslizadores de **Punto de Entrada (In)** y **Punto de Salida (Out)**.
  - Corte ultra-rápido sin recodificación (remuxing directo en el contenedor copiando frames de video y bloques de audio).
  - Botón directo de **"Copiar al Portapapeles"** y **"Abrir Carpeta"**.
- **Crates involucrados:** `rec-core::avi_writer`, `rec-app`.

#### 6. Exportador Instantáneo a GIF de Alta Calidad
- **Propuesta de Valor:** Permite crear clips ligeros de 5 a 15 segundos para documentar bugs en GitHub Issues, pull requests, Slack, Discord y Jira con un solo clic.
- **Detalle Técnico:**
  - Conversión directa de la grabación a formato `.gif` animado.
  - Algoritmo de cuantización de paleta optimizada (NeuQuant o MedianCut con dithering Floyd-Steinberg) para evitar artefactos y bandas de color.
  - Opciones de exportación: 15 / 30 FPS, escala 50% / 75% / 100%, bucle infinito.
- **Crates involucrados:** `rec-core`, `rec-app`.

#### 7. Selector Magnético de Área y Presets para Redes Sociales
- **Propuesta de Valor:** Facilita la grabación inmediata sin necesidad de adivinar resoluciones o encuadres.
- **Detalle Técnico:**
  - Selector de área con snapping magnético automático que detecta las esquinas y límites de cualquier ventana abierta bajo el cursor.
  - Presets preconfigurados con un clic:
    * **16:9 Estándar:** 1080p Full HD (1920x1080), 720p HD (1280x720).
    * **9:16 Vertical:** Optimizado para TikTok, Instagram Reels y YouTube Shorts (1080x1920).
    * **1:1 Cuadrado:** Publicaciones en redes y avatares (1080x1080).
- **Crates involucrados:** `rec-app`, `rec-platform-windows`.

---

### BLOQUE C: Audio Profesional y Claridad de Voz (Studio Audio DSP)

#### 8. Puerta de Ruido y Cancelación de Fondo (Noise Gate DSP)
- **Propuesta de Valor:** Silencia el molesto ruido de fondo (ventiladores del PC, aire acondicionado, tecleo mecánico) cuando el usuario no está hablando.
- **Detalle Técnico:**
  - Procesador DSP en tiempo real en `rec-core::audio`.
  - Parámetros: Umbral de apertura (dB Threshold), tiempo de ataque (Attack time ~5ms) y relajación (Release time ~100ms).
  - Control visual intuitivo en la pestaña de Ajustes de Audio con indicador de umbral sobre el vúmetro en vivo.
- **Crates involucrados:** `rec-core::audio`, `rec-app`.

#### 9. Normalización Automática de Voz (Auto Gain Control / AGC)
- **Propuesta de Valor:** Evita que la voz suene demasiado baja o se distorsione por hablar muy cerca del micrófono.
- **Detalle Técnico:**
  - Algoritmo de nivelación automática que mantiene el volumen del habla entre -3 dB y -6 dB de forma transparente, con limitador de pico suave (soft limiter) para evitar saturación digital.
- **Crates involucrados:** `rec-core::audio`.

---

### BLOQUE D: Rendimiento Extremo y Motor de Video (Engine Turbo)

#### 10. Codificación Directa por Hardware (Media Foundation NVENC / QuickSync / AMF)
- **Propuesta de Valor:** Grabación fluida a 120 FPS en monitores gaming y 4K a 60 FPS con menos del 2% de uso de CPU, exportando directamente en formato MP4 (H.264 / HEVC).
- **Detalle Técnico:**
  - Conexión con Media Foundation Transforms (MFT) por hardware en Windows para GPUs NVIDIA, Intel y AMD.
  - Multiplexado directo a contenedor MP4 con tabla `moov` al inicio (FastStart para streaming web).
- **Crates involucrados:** `rec-platform-windows`, `rec-engine`.

#### 11. Zoom Cinemático Suave (Smooth Auto-Zoom / Focal Zoom)
- **Propuesta de Valor:** La función más codiciada de Screen Studio ($89). Realiza un zoom cinematográfico suave centrado en la zona de clic o en la ventana activa, dando a los videos un aspecto dinámico y profesional.
- **Detalle Técnico:**
  - Seguimiento de eventos de clic y cálculo de la Región de Interés (ROI).
  - Interpolación cúbica fluida (easing curve cubic in-out) de la cámara virtual sobre los fotogramas antes de la codificación.
- **Crates involucrados:** `rec-engine`, `rec-core`.

---

## 4. Estrategia de Comercialización y Modelos de Ingresos

| Nivel (Tier) | Precio Propuesto | Funcionalidades Incluidas |
| :--- | :--- | :--- |
| **Screen Recorder Free (Comunitario)** | **$0 (Gratis para siempre)** | • Grabación ilimitada sin marcas de agua.<br>• Resolución 1080p hasta 60 FPS.<br>• Audio de sistema WASAPI + Micrófono.<br>• HUD flotante, atajos de teclado y exportación AVI. |
| **Screen Recorder Pro (Licencia Personal)** | **$29 (Pago único / Lifetime)** | • Todo lo de Free, más:<br>• **Overlay de Webcam PiP circular**.<br>• **Efectos de cursor (halo y ondas de clic)**.<br>• **Herramientas de anotación y dibujo en vivo**.<br>• **Visualizador de teclas (Keycast)**.<br>• **Exportador instantáneo a GIF HD**.<br>• **Editor rápido de corte (Quick Trim)**. |
| **Screen Recorder Pro Studio (Comercial)** | **$49 (Pago único / 2 puestos)** | • Todo lo de Pro, más:<br>• **Smooth Zoom Cinemático**.<br>• **Codificación por hardware NVENC/QSV 120 FPS / 4K**.<br>• **Filtros de audio DSP (Noise Gate & AGC)**.<br>• Soporte prioritario y actualizaciones mayores incluidas. |

---

## 5. Cronograma de Implementación de Alto Valor

```
OCTUBRE 2026       v0.1.11  ▶ Sincronización A/V lockstep 0ms, Fuentes Segoe UI, Escalado UI [COMPLETADO]
NOVIEMBRE 2026     v0.2.0   ▶ Quick Trim Post-Grabación + Exportador a GIF HD + Halo/Ondas de Cursor
DICIEMBRE 2026     v0.3.0   ▶ Webcam PiP Flotante + Anotaciones en Pantalla en Vivo + Keycast
ENERO 2027         v0.4.0   ▶ DSP de Audio (Noise Gate/Normalización) + Presets 9:16 Shorts
FEBRERO 2027       v0.5.0   ▶ Hardware NVENC/QSV/AMF (MP4) + Smooth Auto-Zoom Cinemático
MARZO 2027         v1.0.0   ▶ Lanzamiento Comercial Global, Sistema de Licencias y Multiplataforma
```
