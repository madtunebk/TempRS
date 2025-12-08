use eframe::egui::{self, Color32};
use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::helpers::{render_track_card, calculate_grid_layout};
use crate::app::playlists::Track;

/// Sort order for history view
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HistorySortOrder {
    RecentFirst,    // Most recently played first (default)
    RecentLast,     // Oldest played first
    TitleAZ,        // Alphabetical by title
    ArtistAZ,       // Alphabetical by artist
}

impl HistorySortOrder {
    pub fn label(&self) -> &str {
        match self {
            Self::RecentFirst => "Recent First",
            Self::RecentLast => "Oldest First",
            Self::TitleAZ => "Title (A-Z)",
            Self::ArtistAZ => "Artist (A-Z)",
        }
    }
}

/// Action to take when interacting with history track grid
#[derive(Debug, Clone, Copy)]
enum HistoryAction {
    PlaySingle(u64),      // Play single track by ID
    PlayAsPlaylist,       // Load all as playlist
}

/// History view - Shows all playback history in grid layout with pagination
pub fn render_history_view(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .show(ui, |ui| {
            ui.add_space(20.0);
            
            // Title (centered)
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("📜 Playback History")
                        .size(24.0)
                        .color(egui::Color32::WHITE)
                        .strong()
                );
            });
            
            ui.add_space(20.0);
            
            // Get total count from database (cache it)
            if app.content.history_total_tracks == 0 {
                app.content.history_total_tracks = app.content.playback_history.get_count() as usize;
            }
            
            // Show empty state if no history
            if app.content.history_total_tracks == 0 {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(
                        egui::RichText::new("📜")
                            .size(64.0)
                            .color(Color32::GRAY),
                    );
                    ui.add_space(15.0);
                    ui.label(
                        egui::RichText::new("No playback history yet")
                            .size(20.0)
                            .color(Color32::GRAY),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Start playing some tracks to build your history")
                            .size(14.0)
                            .color(Color32::DARK_GRAY),
                    );
                });
                return;
            }
            
            // Calculate pagination
            let _total_pages = (app.content.history_total_tracks + app.content.history_page_size - 1) / app.content.history_page_size;
            let offset = app.content.history_page * app.content.history_page_size;
            
            // Get current page of history from database
            let history_records = app.content.playback_history.get_recent_tracks_paginated(
                app.content.history_page_size,
                offset
            );
            
            // Convert to Track objects and apply filter
            let filter_text = app.content.history_search_filter.to_lowercase();
            let mut history_tracks: Vec<Track> = history_records
                .iter()
                .filter(|record| {
                    if filter_text.is_empty() {
                        true
                    } else {
                        record.title.to_lowercase().contains(&filter_text) ||
                        record.artist.to_lowercase().contains(&filter_text) ||
                        record.genre.as_ref().map_or(false, |g| g.to_lowercase().contains(&filter_text))
                    }
                })
                .map(|record| Track {
                    id: record.track_id,
                    title: record.title.clone(),
                    user: crate::app::playlists::User {
                        id: 0,
                        username: record.artist.clone(),
                        avatar_url: None,
                    },
                    duration: record.duration,
                    genre: record.genre.clone(),
                    artwork_url: None, // Will try cache by track_id
                    permalink_url: None,
                    stream_url: None,  // Will fetch when played
                    streamable: Some(true),
                    playback_count: None,
                    access: None,
                    policy: None,
                })
                .collect();
            
            // Apply sorting (database already gives us RecentFirst)
            match app.content.history_sort_order {
                HistorySortOrder::RecentFirst => {
                    // Already sorted by played_at DESC from database
                }
                HistorySortOrder::RecentLast => {
                    history_tracks.reverse(); // Reverse to get oldest first
                }
                HistorySortOrder::TitleAZ => {
                    history_tracks.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
                }
                HistorySortOrder::ArtistAZ => {
                    history_tracks.sort_by(|a, b| a.user.username.to_lowercase().cmp(&b.user.username.to_lowercase()));
                }
            }
            
            // Preload artwork for visible tracks
            preload_history_artwork(app, ui.ctx(), &history_tracks);
            
            // Search/filter bar
            ui.add_space(5.0);
            
            // Calculate same padding as grid for alignment
            let (_, grid_padding) = calculate_grid_layout(ui.available_width(), 220.0, 15.0);
            
            ui.horizontal(|ui| {
                ui.add_space(grid_padding);
                
                // Search icon + input field
                ui.label(egui::RichText::new("🔍").size(18.0));
                ui.add_space(8.0);
                
                let search_response = ui.add_sized(
                    egui::vec2(300.0, 32.0),
                    egui::TextEdit::singleline(&mut app.content.history_search_filter)
                        .hint_text("Filter by title, artist, or genre...")
                        .desired_width(300.0)
                );
                
                // Reset to page 0 when filter changes
                if search_response.changed() {
                    app.content.history_page = 0;
                }
                
                // Show clear button if filter is active
                if !app.content.history_search_filter.is_empty() {
                    ui.add_space(5.0);
                    if ui.button("✖").clicked() {
                        app.content.history_search_filter.clear();
                        app.content.history_page = 0;
                    }
                }
                
                ui.add_space(20.0);
                
                // Sort dropdown
                ui.label(egui::RichText::new("Sort:").size(13.0).color(Color32::from_rgb(180, 180, 180)));
                ui.add_space(5.0);
                
                egui::ComboBox::from_id_salt("history_sort")
                    .selected_text(app.content.history_sort_order.label())
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        changed |= ui.selectable_value(&mut app.content.history_sort_order, HistorySortOrder::RecentFirst, "Recent First").clicked();
                        changed |= ui.selectable_value(&mut app.content.history_sort_order, HistorySortOrder::RecentLast, "Oldest First").clicked();
                        changed |= ui.selectable_value(&mut app.content.history_sort_order, HistorySortOrder::TitleAZ, "Title (A-Z)").clicked();
                        changed |= ui.selectable_value(&mut app.content.history_sort_order, HistorySortOrder::ArtistAZ, "Artist (A-Z)").clicked();
                        
                        if changed {
                            app.content.history_page = 0;
                        }
                    });
            });
            ui.add_space(15.0);
            
            // Show "no results" message if filter is active but no tracks match
            if !app.content.history_search_filter.is_empty() && history_tracks.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.label(
                        egui::RichText::new("🔍")
                            .size(48.0)
                            .color(Color32::GRAY),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("No tracks found")
                            .size(18.0)
                            .color(Color32::from_rgb(200, 200, 200)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("Try a different search term or clear the filter"))
                            .size(13.0)
                            .color(Color32::GRAY),
                    );
                });
                return;
            }
            
            // Render tracks grid
            if let Some(action) = render_history_grid(app, ui, &history_tracks) {
                match action {
                    HistoryAction::PlaySingle(track_id) => {
                        log::info!("[History] Playing single track: {}", track_id);
                        // Find track - if it has no stream_url, fetch from API
                        if let Some(track) = history_tracks.iter().find(|t| t.id == track_id) {
                            if track.stream_url.is_none() {
                                app.fetch_and_play_track(track_id);
                            } else {
                                app.audio.playback_queue.load_tracks(vec![track.clone()]);
                                app.play_track(track_id);
                            }
                        }
                    }
                    HistoryAction::PlayAsPlaylist => {
                        log::info!("[History] Loading all {} history tracks as playlist", history_tracks.len());
                        // Check if any track needs API fetch
                        let needs_fetch = history_tracks.iter().any(|t| t.stream_url.is_none());
                        if needs_fetch {
                            log::info!("[History] Playlist contains DB tracks, fetching full data...");
                            app.fetch_and_play_playlist(history_tracks.iter().map(|t| t.id).collect());
                        } else {
                            app.audio.playback_queue.load_tracks(history_tracks.clone());
                            if let Some(first_track) = app.audio.playback_queue.current_track() {
                                app.play_track(first_track.id);
                            }
                        }
                    }
                }
            }
            
            // Pagination controls (centered)
            ui.add_space(30.0);
            
            crate::ui_components::helpers::render_pagination_controls(
                ui,
                &mut app.content.history_page,
                app.content.history_total_tracks,
                app.content.history_page_size,
            );
            
            ui.add_space(20.0);
        });
}

/// Render history tracks grid (returns action if any)
fn render_history_grid(app: &mut MusicPlayerApp, ui: &mut egui::Ui, tracks: &[Track]) -> Option<HistoryAction> {
    let (items_per_row, padding) = calculate_grid_layout(ui.available_width(), 220.0, 15.0);

    let mut action = None;
    
    for chunk in tracks.chunks(items_per_row) {
        ui.horizontal(|ui| {
            ui.add_space(padding);
            for track in chunk {
                let (clicked, shift_clicked, _right_clicked) = render_track_card(app, ui, track, 220.0);
                if clicked {
                    action = Some(HistoryAction::PlaySingle(track.id));
                } else if shift_clicked {
                    action = Some(HistoryAction::PlayAsPlaylist);
                }
                ui.add_space(15.0);
            }
        });
        ui.add_space(15.0);
    }
    
    action
}

/// Preload artwork for visible history tracks
fn preload_history_artwork(
    app: &mut MusicPlayerApp,
    ctx: &egui::Context,
    tracks: &[Track],
) {
    // Preload all visible tracks (they're already paginated)
    for track in tracks.iter() {
        let track_id = track.id;
        let cache_key = format!("track:{}", track_id);
        
        // Skip if already in memory cache
        if app.ui.thumb_cache.contains_key(&cache_key) {
            continue;
        }
        
        // Try loading from disk cache by track_id (fast, sync)
        if let Some(cached_data) = crate::utils::cache::load_artwork_cache(track_id) {
            if let Ok(img) = image::load_from_memory(&cached_data) {
                let size = [img.width() as _, img.height() as _];
                let image_buffer = img.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                
                let texture = ctx.load_texture(&cache_key, color_image, egui::TextureOptions::LINEAR);
                app.ui.thumb_cache.insert(cache_key, texture);
            }
        }
        // If not in cache, render_track_card will show no_artwork placeholder or gray box
    }
}
