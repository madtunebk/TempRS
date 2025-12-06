// theme.rs - Dark theme for WGSL Shader Editor
use eframe::egui::{
    self, Color32, Context, CornerRadius, FontFamily, FontId, Margin, Visuals,
};

/// Apply dark theme optimized for code editor
pub fn apply_editor_theme(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = Visuals::dark();

    // Editor dark background
    style.visuals.window_fill = Color32::from_rgb(20, 20, 24);
    style.visuals.panel_fill = Color32::from_rgb(20, 20, 24);
    style.visuals.extreme_bg_color = Color32::from_rgb(12, 12, 14);
    style.visuals.code_bg_color = Color32::from_rgb(16, 16, 18);

    // Rounded corners
    style.visuals.window_corner_radius = CornerRadius::same(8);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(4);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(4);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(4);

    // Spacing
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.window_margin = Margin::same(8);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);

    // Default font
    style.override_font_id = Some(FontId::new(14.0, FontFamily::Proportional));

    ctx.set_style(style);
}

/// Apply dark theme optimized for shader viewer/preview
#[allow(dead_code)]
pub fn apply_viewer_theme(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = Visuals::dark();

    // Pure black background for shader preview
    style.visuals.window_fill = Color32::from_rgb(0, 0, 0);
    style.visuals.panel_fill = Color32::from_rgb(0, 0, 0);
    style.visuals.extreme_bg_color = Color32::from_rgb(0, 0, 0);

    // Minimal UI elements
    style.visuals.window_corner_radius = CornerRadius::same(0);
    style.spacing.window_margin = Margin::same(0);

    ctx.set_style(style);
}
