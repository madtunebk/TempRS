use eframe::egui::{self, Color32};
use crate::app::player_app::MusicPlayerApp;
use crate::utils::artwork::load_thumbnail_artwork;

mod recently_played;
mod suggestions;

use recently_played::TrackAction;

/// Home tab - Personalized feed with grid layout (same as search)
pub fn render_home_view(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    // Trigger data fetch on first render only (updates happen when tracks play)
    if !app.content.home_loading && !app.content.home_content.has_content() {
        app.fetch_home_data();
    }
    
    // Preload artwork for visible tracks
    preload_home_artwork(app, ui.ctx());
    
    egui::ScrollArea::vertical()
        .show(ui, |ui| {
            ui.add_space(20.0);
            
            // Show loading state
            if app.content.home_loading && !app.content.home_content.has_content() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.spinner();
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Loading your music...")
                            .size(16.0)
                            .color(Color32::GRAY),
                    );
                });
                return;
            }
            
            // Section 1: Recently Played
            if let Some(action) = recently_played::render_recently_played_section(app, ui) {
                handle_track_action(app, action, &app.content.home_content.recently_played.iter().take(6).cloned().collect::<Vec<_>>());
            }
            
            // Section 2: Suggestions ("More of what you like")
            if let Some(action) = suggestions::render_suggestions_section(app, ui) {
                handle_track_action(app, action, &app.content.home_content.recommendations.iter().take(6).cloned().collect::<Vec<_>>());
            }
        });
}

/// Handle track action (play single or playlist)
fn handle_track_action(app: &mut MusicPlayerApp, action: TrackAction, tracks: &[crate::app::playlists::Track]) {
    match action {
        TrackAction::PlaySingle(track_id) => {
            // Find track - if it has no stream_url, it's from database and needs API fetch
            if let Some(track) = tracks.iter().find(|t| t.id == track_id) {
                if track.stream_url.is_none() {
                    // Track from database - fetch full data from API
                    log::info!("[Home] Track from DB, fetching full data for: {}", track.title);
                    app.fetch_and_play_track(track_id);
                } else {
                    // Track has stream URL - play directly
                    app.audio.playback_queue.load_tracks(vec![track.clone()]);
                    app.play_track(track_id);
                }
            }
        }
        TrackAction::PlayAsPlaylist => {
            // Check if any track needs API fetch
            let needs_fetch = tracks.iter().any(|t| t.stream_url.is_none());
            if needs_fetch {
                log::info!("[Home] Playlist contains DB tracks, fetching full data...");
                app.fetch_and_play_playlist(tracks.iter().map(|t| t.id).collect());
            } else {
                // All tracks have stream URLs - play directly
                app.audio.playback_queue.load_tracks(tracks.to_vec());
                if let Some(first_track) = tracks.first() {
                    app.play_track(first_track.id);
                }
            }
        }
    }
}

/// Preload artwork for visible home screen tracks
/// Simple approach: Just trigger artwork loading for all visible items
/// The artwork system handles caching, deduplication, and progressive loading
fn preload_home_artwork(app: &mut MusicPlayerApp, ctx: &egui::Context) {
    // Collect artwork URLs first to avoid borrow checker issues
    let mut artwork_to_load = Vec::new();
    
    // Recently played artwork
    for track in app.content.home_content.recently_played.iter().take(6) {
        if let Some(artwork_url) = &track.artwork_url {
            let url = artwork_url.replace("-large.jpg", "-t500x500.jpg");
            artwork_to_load.push((track.id, url));
        }
    }
    
    // Recommendations artwork
    for track in app.content.home_content.recommendations.iter().take(6) {
        if let Some(artwork_url) = &track.artwork_url {
            let url = artwork_url.replace("-large.jpg", "-t500x500.jpg");
            artwork_to_load.push((track.id, url));
        }
    }
    
    // Now trigger loading for all collected items
    for (track_id, url) in artwork_to_load {
        load_thumbnail_artwork(app, ctx, track_id, url, false);
    }
}
