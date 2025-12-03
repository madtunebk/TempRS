/// Recommendations section component for Home screen ("More of what you like")
use eframe::egui::{self, Color32};
use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::helpers::{render_section_header, render_track_card, calculate_grid_layout};
use super::recently_played::TrackAction;

/// Render \"Suggestions\" section for Home screen
/// Returns Some(action) if user clicked on a track
pub fn render_recommendations_section(
    app: &mut MusicPlayerApp,
    ui: &mut egui::Ui,
) -> Option<TrackAction> {
    let recommendations = app.home_content.recommendations.clone();
    
    // Always show section header
    if !recommendations.is_empty() {
        if render_section_header(ui, "Suggestions", Some("See more")) {
            log::info!("See more clicked for suggestions - navigating to Suggestions tab");
            app.selected_tab = crate::app::player_app::MainTab::Suggestions;
        }
    } else {
        render_section_header(ui, "Suggestions", None);
    }
    ui.add_space(20.0);
    
    if !recommendations.is_empty() {
        
        // Limit to 6 items (1 row)
        let limited_tracks: Vec<_> = recommendations.iter().take(6).cloned().collect();
        
        // Render grid and get action
        let action = render_tracks_grid(app, ui, &limited_tracks);
        
        ui.add_space(50.0);
        
        action
    } else if app.home_recommendations_loading {
        // Show loading state for recommendations
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.spinner();
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Finding music you might like...")
                    .size(14.0)
                    .color(Color32::GRAY),
            );
        });
        ui.add_space(40.0);
        
        None
    } else {
        // Show gray placeholder cards when no recommendations
        let (items_per_row, padding) = calculate_grid_layout(ui.available_width(), 220.0, 15.0);
        ui.horizontal(|ui| {
            ui.add_space(padding);
            for _ in 0..6 {
                crate::ui_components::helpers::render_placeholder_card(ui, 220.0);
                ui.add_space(15.0);
            }
        });
        ui.add_space(50.0);
        
        None
    }
}

/// Render tracks grid (returns action if any)
fn render_tracks_grid(
    app: &mut MusicPlayerApp,
    ui: &mut egui::Ui,
    tracks: &[crate::app::playlists::Track],
) -> Option<TrackAction> {
    let (items_per_row, padding) = calculate_grid_layout(ui.available_width(), 220.0, 15.0);

    let mut action = None;
    
    for chunk in tracks.chunks(items_per_row) {
        ui.horizontal(|ui| {
            ui.add_space(padding);
            
            // Render real tracks
            for track in chunk {
                let (clicked, shift_clicked, _right_clicked) = render_track_card(app, ui, track, 220.0);
                if clicked {
                    action = Some(TrackAction::PlaySingle(track.id));
                } else if shift_clicked {
                    action = Some(TrackAction::PlayAsPlaylist);
                }
                ui.add_space(15.0);
            }
            
            // Fill remaining slots in this row with grey placeholders
            let remaining_slots = items_per_row - chunk.len();
            for _ in 0..remaining_slots {
                crate::ui_components::helpers::render_placeholder_card(ui, 220.0);
                ui.add_space(15.0);
            }
        });
        ui.add_space(15.0);
    }
    
    action
}
