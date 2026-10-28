//! Obsidian Dark Theme Design System for Screen Recorder Pro.
//!
//! Conforms to `[SPEC-UIUX-001]` and design tokens extracted from `./UIUX/color_pallette.png`.

use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, Rect, Stroke, Vec2};

/// Primary Electric Blue accent (`#3B82F6`).
pub const COLOR_PRIMARY: Color32 = Color32::from_rgb(0x3B, 0x82, 0xF6);
/// Light Electric Blue hover state (`#60A5FA`).
pub const COLOR_PRIMARY_HOVER: Color32 = Color32::from_rgb(0x60, 0xA5, 0xFA);
/// Secondary Coral / Record Red (`#EF4444`).
pub const COLOR_SECONDARY: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);
/// Secondary Coral Red hover state (`#F87171`).
pub const COLOR_SECONDARY_HOVER: Color32 = Color32::from_rgb(0xF8, 0x71, 0x71);
/// Tertiary Emerald / Ready Green (`#10B981`).
pub const COLOR_TERTIARY: Color32 = Color32::from_rgb(0x10, 0xB9, 0x81);
/// Rust Architecture Orange pill (`#E05625`).
pub const COLOR_RUST_ORANGE: Color32 = Color32::from_rgb(0xE0, 0x56, 0x25);

/// Deep Obsidian neutral canvas background (`#0D1117`).
pub const BG_CANVAS: Color32 = Color32::from_rgb(0x0D, 0x11, 0x17);
/// Card and panel surface background (`#161B22`).
pub const BG_CARD: Color32 = Color32::from_rgb(0x16, 0x1B, 0x22);
/// Card hover surface background (`#1F242C`).
pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(0x1F, 0x24, 0x2C);
/// Elevated input, dropdown and sub-card background (`#21262D`).
pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x21, 0x26, 0x2D);

/// Subtle card and container border stroke (`#30363D`).
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x30, 0x36, 0x3D);
/// Active / focused element border stroke (`#388BFD`).
pub const BORDER_FOCUS: Color32 = Color32::from_rgb(0x38, 0x8B, 0xFD);

/// Pure headline white text (`#F0F6FC`).
pub const TEXT_HEADLINE: Color32 = Color32::from_rgb(0xF0, 0xF6, 0xFC);
/// Readable body text (`#C9D1D9`).
pub const TEXT_BODY: Color32 = Color32::from_rgb(0xC9, 0xD1, 0xD9);
/// Subdued muted secondary text (`#8B949E`).
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x8B, 0x94, 0x9E);

/// Configures the global `egui::Context` with the Obsidian design system.
pub fn configure_obsidian_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = BG_CANVAS;
    visuals.window_fill = BG_CARD;
    visuals.faint_bg_color = BG_CARD;
    visuals.extreme_bg_color = BG_ELEVATED;

    visuals.window_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    visuals.window_corner_radius = CornerRadius::same(10);

    // Interactive Widget Styling
    visuals.widgets.inactive.bg_fill = BG_CARD;
    visuals.widgets.inactive.weak_bg_fill = BG_CARD;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_BODY);

    visuals.widgets.hovered.bg_fill = BG_CARD_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_CARD_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_FOCUS);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_HEADLINE);

    visuals.widgets.active.bg_fill = BG_ELEVATED;
    visuals.widgets.active.weak_bg_fill = BG_ELEVATED;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5, COLOR_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_HEADLINE);

    visuals.selection.bg_fill = COLOR_PRIMARY;
    visuals.selection.stroke = Stroke::new(1.0, COLOR_PRIMARY_HOVER);

    ctx.set_visuals(visuals);
}

/// Installs native system fonts (Segoe UI and Segoe UI Symbol on Windows) for crisp typography and glyph support.
pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    #[cfg(target_os = "windows")]
    {
        if let Ok(segoe) = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
            fonts.font_data.insert(
                "segoeui".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(segoe)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "segoeui".to_owned());
        }

        if let Ok(seguisym) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
            fonts.font_data.insert(
                "seguisym".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(seguisym)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("seguisym".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}


/// Standard card container frame conforming to `[REQ-UIUX-001]`.
pub fn card_frame() -> Frame {
    Frame::new()
        .fill(BG_CARD)
        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(12))
}

/// Elevated input / control frame.
pub fn elevated_frame() -> Frame {
    Frame::new()
        .fill(BG_ELEVATED)
        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 6))
}

/// Renders a live dual/single VU meter level bar in Emerald Green.
pub fn render_vu_meter(ui: &mut egui::Ui, peak: f32, width: f32, height: f32) {
    let (rect, _response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let painter = ui.painter();

    // Background track
    painter.rect_filled(rect, CornerRadius::same(height as u8 / 2), BG_ELEVATED);

    // Active fill clamped between 0.0 and 1.0
    let clamped_peak = peak.clamp(0.0, 1.0);
    if clamped_peak > 0.005 {
        let active_width = (rect.width() * clamped_peak).max(height);
        let active_rect = Rect::from_min_size(rect.min, Vec2::new(active_width, height));
        
        let fill_color = if clamped_peak > 0.85 {
            COLOR_SECONDARY // Red peak warning
        } else if clamped_peak > 0.65 {
            Color32::from_rgb(0xF5, 0x9E, 0x0B) // Amber high level
        } else {
            COLOR_TERTIARY // Emerald green nominal
        };

        painter.rect_filled(active_rect, CornerRadius::same(height as u8 / 2), fill_color);
    }
}

/// Renders a pill badge widget with custom background and text color.
pub fn render_badge(ui: &mut egui::Ui, text: &str, bg_color: Color32, text_color: Color32) {
    Frame::new()
        .fill(bg_color)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui: &mut egui::Ui| {
            ui.colored_label(text_color, egui::RichText::new(text).size(11.0).strong());
        });
}
