use eframe::egui;
use crate::app::player_app::{MusicPlayerApp, MainTab};
use crate::ui_components::colors::*;

// UI Constants
const BUTTON_HEIGHT: f32 = 34.0;
const CORNER_RADIUS: f32 = 3.0;

pub fn render_header(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    // Check for drag on the entire header background for window movement
    let header_rect = ui.available_rect_before_wrap();
    let header_response = ui.interact(header_rect, ui.id().with("header_drag"), egui::Sense::click_and_drag());
    
    if header_response.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 15.0;
        ui.add_space(20.0);

        // Navigation
        render_navigation_icons(app, ui);
        crate::ui_components::search_bar::render_search_section(app, ui);
        render_user_section(app, ui);
    });
}

/// Render icon-based navigation with tooltips and text labels
fn render_navigation_icons(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.x = 8.0;
    ui.add_space(5.0);
    
    // Home icon with text (always visible)
    let home_active = app.ui.selected_tab == MainTab::Home;
    let home_color = if home_active { ORANGE } else { LIGHT_GRAY };
    
    let home_btn = ui.add_sized(
        egui::vec2(70.0, BUTTON_HEIGHT),
        egui::Button::new(
            egui::RichText::new("Home")
                .size(14.0)
                .color(home_color)
        )
        .fill(if home_active { MID_GRAY } else { DARK_GRAY })
        .corner_radius(CORNER_RADIUS)
    ).on_hover_text("Home (O)");
    
    if home_btn.clicked() {
        app.ui.selected_tab = MainTab::Home;
    }
    
    // History icon with text (always visible)
    let history_active = app.ui.selected_tab == MainTab::History;
    let history_color = if history_active { ORANGE } else { LIGHT_GRAY };
    
    let history_btn = ui.add_sized(
        egui::vec2(75.0, BUTTON_HEIGHT),
        egui::Button::new(
            egui::RichText::new("History")
                .size(14.0)
                .color(history_color)
        )
        .fill(if history_active { MID_GRAY } else { DARK_GRAY })
        .corner_radius(CORNER_RADIUS)
    ).on_hover_text("History (H)");
    
    if history_btn.clicked() {
        app.ui.selected_tab = MainTab::History;
    }
    
    // Suggestions icon with text (always visible)
    let suggestions_active = app.ui.selected_tab == MainTab::Suggestions;
    let suggestions_color = if suggestions_active { ORANGE } else { LIGHT_GRAY };
    
    let suggestions_btn = ui.add_sized(
        egui::vec2(105.0, BUTTON_HEIGHT),
        egui::Button::new(
            egui::RichText::new("Suggestions")
                .size(14.0)
                .color(suggestions_color)
        )
        .fill(if suggestions_active { MID_GRAY } else { DARK_GRAY })
        .corner_radius(CORNER_RADIUS)
    ).on_hover_text("Suggestions (S)");
    
    if suggestions_btn.clicked() {
        app.ui.selected_tab = MainTab::Suggestions;
        // Refresh suggestions data on every click
        app.fetch_all_suggestions();
    }
    
    // Likes icon with text (always visible)
    let likes_active = app.ui.selected_tab == MainTab::Likes;
    let likes_color = if likes_active { ORANGE } else { LIGHT_GRAY };
    
    let likes_btn = ui.add_sized(
        egui::vec2(60.0, BUTTON_HEIGHT),
        egui::Button::new(
            egui::RichText::new("Likes")
                .size(14.0)
                .color(likes_color)
        )
        .fill(if likes_active { MID_GRAY } else { DARK_GRAY })
        .corner_radius(CORNER_RADIUS)
    ).on_hover_text("Likes (L)");
    
    if likes_btn.clicked() {
        app.ui.selected_tab = MainTab::Likes;
        // Refresh likes data on every click
        app.fetch_likes();
    }
    
    // Playlists icon with text (always visible)
    let playlists_active = app.ui.selected_tab == MainTab::Playlists;
    let playlists_color = if playlists_active { ORANGE } else { LIGHT_GRAY };
    
    let playlists_btn = ui.add_sized(
        egui::vec2(85.0, BUTTON_HEIGHT),
        egui::Button::new(
            egui::RichText::new("Playlists")
                .size(14.0)
                .color(playlists_color)
        )
        .fill(if playlists_active { MID_GRAY } else { DARK_GRAY })
        .corner_radius(CORNER_RADIUS)
    ).on_hover_text("Playlists (P)");
    
    if playlists_btn.clicked() {
        app.ui.selected_tab = MainTab::Playlists;
        // Refresh playlists data on every click
        app.fetch_playlists();
    }
    
    // Search Results icon with text (only show when search results exist)
    if app.ui.selected_tab == MainTab::Search || (!app.content.search_results_tracks.is_empty() || !app.content.search_results_playlists.is_empty()) {
        let results_active = app.ui.selected_tab == MainTab::Search;
        let results_color = if results_active { ORANGE } else { LIGHT_GRAY };
        
        let results_btn = ui.add_sized(
            egui::vec2(75.0, BUTTON_HEIGHT),
            egui::Button::new(
                egui::RichText::new("Results")
                    .size(14.0)
                    .color(results_color)
            )
            .fill(if results_active { MID_GRAY } else { DARK_GRAY })
            .corner_radius(CORNER_RADIUS)
        ).on_hover_text("Search Results (R)");
        
        if results_btn.clicked() {
            app.ui.selected_tab = MainTab::Search;
        }
    }
    
    // Now Playing icon with text (only show when track is active)
    if app.audio.current_track_id.is_some() {
        let now_playing_active = app.ui.selected_tab == MainTab::NowPlaying;
        let icon_color = if now_playing_active { ORANGE } else { LIGHT_GRAY };
        
        let np_btn = ui.add_sized(
            egui::vec2(110.0, BUTTON_HEIGHT),
            egui::Button::new(
                egui::RichText::new("Now Playing")
                    .size(14.0)
                    .color(icon_color)
            )
            .fill(if now_playing_active { MID_GRAY } else { DARK_GRAY })
            .corner_radius(CORNER_RADIUS)
        ).on_hover_text("Now Playing (N)");
        
        if np_btn.clicked() {
            app.ui.selected_tab = MainTab::NowPlaying;
        }
    }
}

/// Render user avatar and logout button
fn render_user_section(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(20.0);
        
        // Logout icon button
        let logout_btn = ui.add_sized(
            egui::vec2(BUTTON_HEIGHT, BUTTON_HEIGHT),
            egui::Button::new(
                egui::RichText::new("⏻")
                    .size(20.0)
                    .color(LIGHT_GRAY)
            )
            .fill(DARK_GRAY)
            .corner_radius(CORNER_RADIUS)
        ).on_hover_text("Logout");
        
        if logout_btn.clicked() {
            app.logout();
        }
        
        ui.add_space(10.0);
        
        // Profile avatar
        if let Some(avatar_texture) = &app.auth.user_avatar_texture {
            ui.add(
                egui::Image::new(avatar_texture)
                    .fit_to_exact_size(egui::vec2(32.0, 32.0))
                    .corner_radius(16.0)
            );
        } else {
            ui.add(
                egui::Label::new(
                    egui::RichText::new("●")
                        .size(20.0)
                        .color(LIGHT_GRAY)
                )
            );
        }
    });
}

