use eframe::egui;
use std::fs;
use std::path::Path;

/// Configure fonts for the application:
/// - UI: Inter (professional, modern, excellent readability)
/// - Errors: RobotoMono (monospace, great Unicode support)
/// - Code Editor: Default monospace (handled by egui_code_editor)
pub fn register_error_fonts(ctx: &egui::Context) {
    let inter_regular = Path::new("src/assets/fonts/static/Inter-Regular.ttf");
    let inter_medium = Path::new("src/assets/fonts/static/Inter-Medium.ttf");
    let roboto_mono = Path::new("src/assets/fonts/static/RobotoMono-Regular.ttf");

    let mut defs = egui::FontDefinitions::default();
    let mut fonts_loaded = false;

    // Add Inter for UI (professional, modern font)
    if inter_regular.exists() && inter_medium.exists() {
        if let (Ok(regular_bytes), Ok(medium_bytes)) = (fs::read(inter_regular), fs::read(inter_medium)) {
            defs.font_data.insert(
                "Inter-Regular".to_owned(),
                egui::FontData::from_owned(regular_bytes).into(),
            );
            defs.font_data.insert(
                "Inter-Medium".to_owned(),
                egui::FontData::from_owned(medium_bytes).into(),
            );

            // Use Inter as primary proportional font for UI
            defs.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Inter-Regular".to_owned());

            fonts_loaded = true;
        }
    }

    // Add RobotoMono for error messages and monospace needs
    if roboto_mono.exists() {
        if let Ok(bytes) = fs::read(roboto_mono) {
            defs.font_data.insert(
                "RobotoMono".to_owned(),
                egui::FontData::from_owned(bytes).into(),
            );

            // Use RobotoMono as primary monospace font (for errors)
            defs.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "RobotoMono".to_owned());

            fonts_loaded = true;
        }
    }

    if fonts_loaded {
        ctx.set_fonts(defs);

        // Configure text styles with better sizing
        let mut style = (*ctx.style()).clone();

        // UI text - Inter font (clean, professional)
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::proportional(15.0)
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::proportional(15.0)
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::proportional(20.0)
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::proportional(13.0)
        );

        // Monospace for errors and debugging - RobotoMono
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::monospace(13.0)
        );

        ctx.set_style(style);
    }
}
