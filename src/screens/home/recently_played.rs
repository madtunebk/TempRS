/// Recently Played section component for Home screen
use eframe::egui;
use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::helpers::{render_section_header, render_track_card, calculate_grid_layout};

/// Action to take when interacting with track grid
#[derive(Debug, Clone, Copy)]
pub enum TrackAction {
    PlaySingle(u64),      // Play single track by ID
    PlayAsPlaylist,       // Load all as playlist
}

/// Render "Recently Played" section with up to 6 tracks
/// Returns Some(action) if user clicked on a track
pub fn render_recently_played_section(
    app: &mut MusicPlayerApp,
    ui: &mut egui::Ui,
) -> Option<TrackAction> {
    let recently_played = app.home_content.recently_played.clone();
    
    // Always show section header
    if !recently_played.is_empty() {
        if render_section_header(ui, "🕒 Recently Played", Some("View all")) {
            log::info!("View all clicked for Recently Played - switching to History tab");
            app.selected_tab = crate::app::player_app::MainTab::History;
        }
    } else {
        render_section_header(ui, "🕒 Recently Played", None);
    }
    ui.add_space(15.0);
    
    // Show empty state if no tracks
    if recently_played.is_empty() {
        crate::ui_components::helpers::render_empty_state(
            ui,
            "🎵",
            "No recently played tracks",
            "Start playing music to see it here",
        );
        ui.add_space(40.0);
        return None;
    }
    
    // Limit to 6 items (1 row)
    let limited_tracks: Vec<_> = recently_played.iter().take(6).cloned().collect();
    
    // Render grid and get action
    let action = render_tracks_grid(app, ui, &limited_tracks);
    
    ui.add_space(40.0);
    
    action
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
        ui.add_space(12.0);
    }
    
    action
}
