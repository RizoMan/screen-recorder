# SPECIFICATION: UI/UX DESIGN SYSTEM & OBSIDIAN DARK THEME
Document ID: `[SPEC-UIUX-001]`
Status: APPROVED / MASTER DESIGN SYSTEM
Domain: User Interface, Design Tokens & Component Library
Version: 1.0.0

---

## 1. Overview & Source Anchors

Esta especificación formaliza los tokens visuales, la jerarquía tipográfica, el sistema de espaciado y la biblioteca de componentes interactivos extraídos directamente de los artefactos de diseño provistos en `./UIUX`:
- `./UIUX/color_pallette.png`: Tokens de color, gradaciones y estilos de botones.
- `./UIUX/main_screen.png`: Pantalla principal del grabador con tarjetas de medios, selección de pantalla, barra de destino y botones de acción.
- `./UIUX/settings_screen_1.png`: Panel modal de configuración con barra lateral, selectores de codificadores por hardware y telemetría en tiempo real.

---

## 2. Design Tokens `[REQ-UIUX-001]`

### 2.1 Paleta de Colores Primaria y Semántica
```rust
// Tokens de Color Primarios (Extraídos de ./UIUX/color_pallette.png)
pub const COLOR_PRIMARY: Color32 = Color32::from_rgb(0x3B, 0x82, 0xF6);       // #3B82F6 Electric Blue
pub const COLOR_PRIMARY_HOVER: Color32 = Color32::from_rgb(0x60, 0xA5, 0xFA); // #60A5FA Light Blue
pub const COLOR_SECONDARY: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);     // #EF4444 Coral / Record Red
pub const COLOR_SECONDARY_HOVER: Color32 = Color32::from_rgb(0xF8, 0x71, 0x71); // #F87171 Record Red Hover
pub const COLOR_TERTIARY: Color32 = Color32::from_rgb(0x10, 0xB9, 0x81);      // #10B981 Emerald / Ready Green
pub const COLOR_RUST_ORANGE: Color32 = Color32::from_rgb(0xE0, 0x56, 0x25);   // Rust Core Pill

// Superficies y Fondos Neutros (Deep Obsidian)
pub const BG_APP: Color32 = Color32::from_rgb(0x0D, 0x11, 0x17);              // #0D1117 Main Canvas Neutral
pub const BG_CARD: Color32 = Color32::from_rgb(0x16, 0x1B, 0x22);             // #161B22 Card Container
pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(0x1F, 0x24, 0x2C);       // #1F242C Interactive Hover
pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x21, 0x26, 0x2D);         // #21262D Input / Subcard

// Bordes y Separadores
pub const BORDER_CARD: Color32 = Color32::from_rgb(0x30, 0x36, 0x3D);         // #30363D Normal Border
pub const BORDER_FOCUS: Color32 = Color32::from_rgb(0x38, 0x8B, 0xFD);        // #388BFD Active Border

// Tipografía y Texto
pub const TEXT_HEADLINE: Color32 = Color32::from_rgb(0xF0, 0xF6, 0xFC);       // #F0F6FC Pure White/Light
pub const TEXT_BODY: Color32 = Color32::from_rgb(0xC9, 0xD1, 0xD9);           // #C9D1D9 Subtle Body
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x8B, 0x94, 0x9E);          // #8B949E Muted / Subtitle
```

### 2.2 Geometría y Radios de Borde (Corner Radii)
- **Tarjetas y Contenedores:** `CornerRadius::same(10)`
- **Botones y Controles Interactivos:** `CornerRadius::same(6)`
- **Insignias y Pills de Estado:** `CornerRadius::same(14)`

---

## 3. Arquitectura de Pantalla Principal (`main_screen.png`) `[REQ-UIUX-002]`

1. **Barra Superior (Top Navigation Header):**
   - Ícono de diana con fondo cuadrado redondeado (`#3B82F6`).
   - Título: **Screen Recorder**.
   - Insignia de producto: `PRO` (azul eléctrico).
   - Estado del motor: `● Listo` (verde esmeralda `#10B981`).
   - Insignia de arquitectura: `🦀 Core v2.4 (Sub-5ms)`.
   - Navegación: `General`, `⚙ Ajustes`, y selector `🌐 ES / EN`.
2. **Barra de Selección de Modo (Capture Mode Bar):**
   - Grupo de botones segmentados: `🖥 Monitor Completo` (activo), `🗔 Ventana`, `⬚ Región Libre`, `⚙ Presets`.
   - Lado derecho: etiqueta de resolución `1080p (1920x1080)` y pastilla de fotogramas `60.00 FPS`.
3. **Tarjeta de Selección de Monitor:**
   - Ícono de laptop/monitor número `1`.
   - Encabezado `PANTALLA PRIMARIA` en gris tenue.
   - Selector desplegable estilizado: `\\.\DISPLAY1 [1920x1080] ★ Principal (sRGB)`.
   - Lado derecho: etiqueta `Backend de Grabación:` con pastilla `Windows Graphics Capture (WGC)`.
4. **Fila de Fuentes de Medios (3 Tarjetas en Columna):**
   - **Audio del Sistema:** Nivel de volumen (`100%`), barra de vúmetro estéreo en verde y subtítulo con nombre del dispositivo (`Altavoces...`).
   - **Micrófono:** Botón de silenciar, barra de vúmetro en vivo y subtítulo con nombre del micrófono (`Micrófono USB...`).
   - **Cámara Web (PIP):** Interruptor deslizante on/off y pastilla `Encoder HW: NVENC H.264 / D3D11`.
5. **Barra de Destino y Salida:**
   - `📁 Destino:` con ruta actual en caja estilizada oscura.
   - Botón `... Cambiar` y botón `📂 Abrir Carpeta`.
   - Selector de formato: `MP4 ⌵` / `AVI ⌵`.
6. **Botones de Acción Heroica:**
   - Botón Gigante de Grabación (`#EF4444` Coral Red): `● Iniciar Grabación` con tecla de acceso rápido `F9`. Al estar grabando, transmuta a `■ Detener Grabación` con pulso dinámico.
   - Botón Secundario (`#1F242C` Slate): `📷 Captura de Imagen` con tecla `F10`.
7. **Tarjeta de Grabación Reciente:**
   - Ícono de video en squircle verde.
   - Nombre de archivo con pastilla `Reciente`.
   - Metadatos en una línea: `⏱ 04:22 min  •  💾 142.8 MB`.
   - Botones rápidos: `▶ Reproducir` (azul primario) y `📁 Mostrar`.
8. **Barra de Estado Inferior:**
   - Telemetría de hardware en tiempo real: `⚙ CPU: 1.4%   RAM: 64 MB`.
   - Estado del motor: `Motor de Captura Listo`.

---

## 4. Arquitectura del Panel de Configuración (`settings_screen_1.png`) `[REQ-UIUX-003]`

1. **Encabezado Modal:**
   - Título: `Configuración / Ajustes del Encoder`.
   - Badges: `NVENC / Rust-AV Core v2.4` y `● Hardware Ready (Sub-3ms)`.
2. **Barra Lateral Izquierda de Categorías:**
   - `🎞 Video & Encoder` (seleccionado por defecto).
   - `🔊 Audio & Canales`.
   - `📁 Salida y Formato`.
   - `⌨ Atajos de Teclado`.
   - `⚡ Rendimiento & Buffer`.
   - Telemetría en vivo en la parte inferior:
     - `LATENCIA MOTOR: 2.8 ms` (verde)
     - `FRAMES PERDIDOS: 0 (0.00%)`
     - `VRAM USAGE: 412 MB`
3. **Contenido de la Sección Video & Encoder:**
   - Cuadrícula de 4 codificadores:
     - `NVIDIA NVENC` con pastilla `RECOMENDADO`.
     - `AMD AMF Core` con pastilla `VCN 4.0`.
     - `Intel QuickSync` con pastilla `QSV Gen12`.
     - `Software x264 (CPU)` con pastilla `CPU RUST`.
   - Barra de detección de GPU: `🖥 GPU Detectada: NVIDIA GeForce RTX ...`.
   - Modo de Control de Tasa (Bitrate): Segmentos `[CBR (Constante)] [VBR (Variable)] [CQP / CQ]`.
   - Slider de Tasa de Bits Objetivo: `12,000 Kbps (12.0 Mbps)` con marcas visuales de 720p, 1080p, 2K, 4K.
   - Presets de NVENC: `[P1 (Rápido)] [P4 (Medio)] [P7 (Máx. Calidad)]`.
   - Perfil: `High (AVC/H.264)`.
   - Intervalo de Keyframes: `2s (120 frames)`.
4. **Pie de Página (Footer):**
   - `↺ Restablecer Valores por Defecto`.
   - `Cancelar` y `✓ Guardar Cambios`.
