use eframe::egui;

pub fn settings_overlay(
    ctx: &egui::Context,
    show_settings: &mut bool,
    editor_font_size: &mut f32,
) {
    if !*show_settings {
        return;
    }

    // Semi-transparent dark overlay to dim the background
    egui::Area::new(egui::Id::new("modal_overlay"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen_rect = ctx.viewport_rect();
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_black_alpha(180),
            );
        });

    // Center modal window
    egui::Window::new("⚙ Settings")
        .id(egui::Id::new("settings_window"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .resizable(false)
        .collapsible(false)
        .fixed_size(egui::vec2(350.0, 0.0))
        .show(ctx, |ui| {
            ui.add_space(5.0);

            // Editor Section
            ui.push_id("editor_section", |ui| {
                ui.group(|ui| {
                    ui.set_min_width(320.0);
                    ui.heading("📝 Editor");
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("Font Size:");
                        ui.add_space(10.0);
                        ui.add(egui::Slider::new(editor_font_size, 10.0..=24.0).text("px"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Current:");
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!("{}px", *editor_font_size as i32))
                                .monospace()
                                .strong(),
                        );
                    });
                });
            });

            ui.add_space(10.0);

            // Close button
            ui.vertical_centered(|ui| {
                if ui.button(egui::RichText::new("Close").size(15.0)).clicked() {
                    *show_settings = false;
                }
            });

            ui.add_space(5.0);
        });
}
