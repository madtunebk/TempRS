use eframe::egui;
use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::colors::*;

// UI Constants
const BUTTON_HEIGHT: f32 = 34.0;
const CORNER_RADIUS: f32 = 3.0;
const SEARCH_WIDTH: f32 = 220.0;

/// Render search bar with integrated type selector
pub fn render_search_section(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.x = 10.0;
    
    if app.search_expanded {
        // Expanded search bar - height matches navigation buttons
        egui::Frame::NONE
            .fill(DARK_GRAY)
            .corner_radius(CORNER_RADIUS)
            .inner_margin(egui::Margin::symmetric(12.0 as i8, 0.0 as i8))
            .show(ui, |ui| {
                ui.set_height(BUTTON_HEIGHT);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    
                    render_search_input(app, ui);
                    ui.separator();
                    ui.add_space(5.0);
                    render_search_type_selector(app, ui);
                    
                    // X button to collapse search
                    ui.add_space(5.0);
                    let close_btn = ui.add_sized(
                        egui::vec2(BUTTON_HEIGHT - 8.0, BUTTON_HEIGHT - 8.0),
                        egui::Button::new(
                            egui::RichText::new("✕")
                                .size(16.0)
                                .color(egui::Color32::from_rgb(180, 180, 180))
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .corner_radius(CORNER_RADIUS)
                    );
                    
                    if close_btn.clicked() {
                        app.search_expanded = false;
                    }
                });
            });
    } else {
        // Collapsed - just show search icon button
        let search_icon_btn = ui.add_sized(
            egui::vec2(BUTTON_HEIGHT, BUTTON_HEIGHT),
            egui::Button::new(
                egui::RichText::new("🔍")
                    .size(14.0)
            )
            .fill(DARK_GRAY)
            .corner_radius(CORNER_RADIUS)
        );
        
        if search_icon_btn.clicked() {
            app.search_expanded = true;
        }
    }
}

/// Render search text input
fn render_search_input(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    let search_input = ui.add_sized(
        egui::vec2(SEARCH_WIDTH, BUTTON_HEIGHT),
        egui::TextEdit::singleline(&mut app.search_query)
            .hint_text("Search SoundCloud...")
            .font(egui::FontId::proportional(14.0))
            .frame(false)
    );
    
    if search_input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        crate::screens::search::trigger_search(app);
    }
}

/// Render search type checkboxes (Tracks/Playlists)
fn render_search_type_selector(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    let tracks_checked = app.search_type == crate::app::player_app::SearchType::Tracks;
    if ui.checkbox(&mut tracks_checked.clone(), 
        egui::RichText::new("Tracks").size(12.0)
    ).clicked() {
        app.search_type = crate::app::player_app::SearchType::Tracks;
        if !app.search_query.is_empty() {
            crate::screens::search::trigger_search(app);
        }
    }
    
    let playlists_checked = app.search_type == crate::app::player_app::SearchType::Playlists;
    if ui.checkbox(&mut playlists_checked.clone(), 
        egui::RichText::new("Playlists").size(12.0)
    ).clicked() {
        app.search_type = crate::app::player_app::SearchType::Playlists;
        if !app.search_query.is_empty() {
            crate::screens::search::trigger_search(app);
        }
    }
}
