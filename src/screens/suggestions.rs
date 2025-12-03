use eframe::egui::{self, Color32};
use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::helpers::{render_track_card, calculate_grid_layout};
use crate::app::playlists::Track;

/// Action to take when interacting with suggestions track grid
#[derive(Debug, Clone, Copy)]
enum SuggestionsAction {
    PlaySingle(u64),      // Play single track by ID
    PlayAsPlaylist,       // Load all as playlist
}

/// Suggestions view - Shows personalized recommendations in grid layout with pagination
pub fn render_suggestions_view(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .show(ui, |ui| {
            ui.add_space(20.0);
            
            // Trigger initial fetch if needed
            if !app.suggestions_loading && app.suggestions_tracks.is_empty() && !app.suggestions_initial_fetch_done {
                app.fetch_all_suggestions();
            }
            
            // Show loading state
            if app.suggestions_loading && app.suggestions_tracks.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.spinner();
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Loading personalized suggestions...")
                            .size(16.0)
                            .color(Color32::GRAY),
                    );
                });
                return;
            }
            
            // Show empty state if no suggestions
            if app.suggestions_tracks.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(
                        egui::RichText::new("✨")
                            .size(64.0)
                            .color(Color32::GRAY),
                    );
                    ui.add_space(15.0);
                    ui.label(
                        egui::RichText::new("No suggestions yet")
                            .size(20.0)
                            .color(Color32::GRAY),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Start listening to music to discover personalized recommendations")
                            .size(14.0)
                            .color(Color32::DARK_GRAY),
                    );
                });
                return;
            }
            
            // Show all tracks (no pagination)
            let total_suggestions = app.suggestions_tracks.len();
            let all_tracks = app.suggestions_tracks.clone();
            
            // Preload artwork for visible tracks
            preload_suggestions_artwork(app, ui.ctx(), &all_tracks);
            
            // Header with track count
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                let header_text = format!("✨ Suggestions for You ({} tracks)", total_suggestions);
                ui.label(
                    egui::RichText::new(&header_text)
                        .size(24.0)
                        .color(egui::Color32::WHITE)
                        .strong()
                );
            });
            
            ui.add_space(20.0);
            
            // Render tracks grid
            if let Some(action) = render_suggestions_grid(app, ui, &all_tracks) {
                match action {
                    SuggestionsAction::PlaySingle(track_id) => {
                        log::info!("[Suggestions] Playing single track: {}", track_id);
                        if let Some(track) = all_tracks.iter().find(|t| t.id == track_id) {
                            app.playback_queue.load_tracks(vec![track.clone()]);
                            app.play_track(track_id);
                        }
                    }
                    SuggestionsAction::PlayAsPlaylist => {
                        log::info!("[Suggestions] Loading all {} suggestions as playlist", all_tracks.len());
                        app.playback_queue.load_tracks(all_tracks.clone());
                        if let Some(first_track) = app.playback_queue.current_track() {
                            app.play_track(first_track.id);
                        }
                    }
                }
            }
            
            ui.add_space(40.0);
        });
}

/// Render suggestions tracks grid (returns action if any)
fn render_suggestions_grid(app: &mut MusicPlayerApp, ui: &mut egui::Ui, tracks: &[Track]) -> Option<SuggestionsAction> {
    let (items_per_row, padding) = calculate_grid_layout(ui.available_width(), 220.0, 15.0);

    let mut action = None;
    
    for chunk in tracks.chunks(items_per_row) {
        ui.horizontal(|ui| {
            ui.add_space(padding);
            for track in chunk {
                let (clicked, shift_clicked, _right_clicked) = render_track_card(app, ui, track, 220.0);
                if clicked {
                    action = Some(SuggestionsAction::PlaySingle(track.id));
                } else if shift_clicked {
                    action = Some(SuggestionsAction::PlayAsPlaylist);
                }
                ui.add_space(15.0);
            }
        });
        ui.add_space(15.0);
    }
    
    action
}

/// Preload artwork for visible suggestions tracks
fn preload_suggestions_artwork(
    app: &mut MusicPlayerApp,
    ctx: &egui::Context,
    tracks: &[Track],
) {
    // Use same system as Search - check artwork_url in memory cache
    for track in tracks.iter() {
        let artwork_url = track
            .artwork_url
            .as_ref()
            .map(|url| url.replace("-large.jpg", "-t500x500.jpg"))
            .unwrap_or_default();
        
        if !artwork_url.is_empty() && !app.thumb_cache.contains_key(&artwork_url) {
            // load_thumbnail_artwork handles disk cache check and async download
            crate::utils::artwork::load_thumbnail_artwork(app, ctx, track.id, artwork_url, false);
        }
    }
}
