use eframe::egui::{self, Vec2, Sense, Color32, CornerRadius};
use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::helpers::{truncate_text, calculate_grid_layout};
use crate::utils::artwork::load_thumbnail_artwork;
use std::sync::mpsc::channel;

/// Render playlists search results grid with pagination
pub fn render_playlists_grid_paginated(app: &mut MusicPlayerApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    if app.search_results_playlists.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.label(
                egui::RichText::new("No playlists found")
                    .size(18.0)
                    .color(Color32::GRAY),
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Try a different search query")
                    .size(14.0)
                    .color(Color32::DARK_GRAY),
            );
        });
        return;
    }

    // Calculate pagination
    let offset = app.search_page * app.search_page_size;
    let end = (offset + app.search_page_size).min(app.search_results_playlists.len());
    
    if offset >= app.search_results_playlists.len() {
        // Reset to first page if out of bounds
        return;
    }
    
    let page_playlists: Vec<_> = app.search_results_playlists[offset..end].to_vec();
    let (items_per_row, padding) = calculate_grid_layout(ui.available_width(), 220.0, 15.0);

    ui.add_space(10.0);

    for chunk in page_playlists.chunks(items_per_row) {
        ui.horizontal(|ui| {
            ui.add_space(padding);
            for playlist in chunk {
                render_playlist_item(app, ui, ctx, playlist, 220.0);
                ui.add_space(15.0);
            }
        });
        ui.add_space(15.0);
    }
}

fn render_playlist_item(
    app: &mut MusicPlayerApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    playlist: &crate::app::playlists::Playlist,
    size: f32,
) {
    let hover_bg = Color32::from_rgb(40, 40, 45);

    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(size, size + 55.0), Sense::click());

    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), hover_bg);
    }

    let artwork_rect =
        egui::Rect::from_min_size(rect.min, Vec2::new(size, size));

    let artwork_url = playlist
        .artwork_url
        .as_ref()
        .map(|url| url.replace("-large.jpg", "-t500x500.jpg"))
        .unwrap_or_default();

    if !artwork_url.is_empty() {
        // Check memory cache first (fast path)
        if let Some(texture) = app.thumb_cache.get(&artwork_url) {
            ui.painter().image(
                texture.id(),
                artwork_rect,
                egui::Rect::from_min_max(
                    egui::pos2(0.0, 0.0),
                    egui::pos2(1.0, 1.0),
                ),
                Color32::WHITE,
            );
        } else {
            // Not in memory - load_thumbnail_artwork will:
            // 1. Check disk cache every frame (fast, sync - appears immediately when downloaded)
            // 2. Download if not in cache (async, only once)
            // 3. Save to cache for future use
            load_thumbnail_artwork(app, ctx, playlist.id, artwork_url.clone(), false);
            // Show placeholder while loading
            super::draw_no_artwork(app, ui, artwork_rect);
        }
    } else {
        super::draw_no_artwork(app, ui, artwork_rect);
    }

    if response.hovered() {
        ui.painter().rect_filled(
            artwork_rect,
            CornerRadius::same(6),
            Color32::from_black_alpha(80),
        );
    }

    if response.clicked() {
        load_playlist(app, playlist);
    }

    let text_rect = egui::Rect::from_min_size(
        artwork_rect.min + Vec2::new(0.0, size + 5.0),
        Vec2::new(size, 50.0),
    );

    ui.painter().text(
        text_rect.min + Vec2::new(5.0, 0.0),
        egui::Align2::LEFT_TOP,
        truncate_text(&playlist.title, 25),
        egui::FontId::proportional(13.0),
        Color32::WHITE,
    );

    ui.painter().text(
        text_rect.min + Vec2::new(5.0, 18.0),
        egui::Align2::LEFT_TOP,
        format!("{} tracks", playlist.track_count),
        egui::FontId::proportional(11.0),
        Color32::GRAY,
    );
}

fn load_playlist(app: &mut MusicPlayerApp, playlist: &crate::app::playlists::Playlist) {
    log::info!(
        "[Search] Loading playlist: {} ({} tracks)",
        playlist.title,
        playlist.track_count
    );

    let has_preview_tracks = !playlist.tracks.is_empty();
    let needs_full_fetch = playlist.track_count > playlist.tracks.len() as u32;

    // Start instantly with preview tracks (if present)
    if has_preview_tracks {
        log::info!(
            "[Search] Starting playback with {} preview tracks",
            playlist.tracks.len()
        );

        // Filter streamable tracks (include database tracks - they'll be fetched on-demand)
        let preview_tracks: Vec<_> = playlist
            .tracks
            .iter()
            .filter(|t| t.streamable.unwrap_or(false))
            .cloned()
            .collect();

        if !preview_tracks.is_empty() {
            app.playback_queue.load_tracks(preview_tracks);

            if let Some(first_track) = app.playback_queue.current_track() {
                let track_id = first_track.id;
                app.play_track(track_id);
            }
        }
    } else if needs_full_fetch {
        // No preview tracks, clear queue and prepare for chunked loading
        log::info!("[Search] No preview tracks, clearing queue for fresh load");
        app.playback_queue.load_tracks(Vec::new());
    }

    // If playlist is larger than preview, fetch full content in chunks
    if needs_full_fetch {
        log::info!(
            "[Search] Starting chunked fetch for {} total tracks",
            playlist.track_count
        );

        let playlist_id = playlist.id;
        let token = match app.app_state.get_token() {
            Some(t) => t,
            None => {
                log::error!(
                    "[Search] No token available for fetching full playlist"
                );
                return;
            }
        };

        let (tx, rx) = channel();
        app.playlist_chunk_rx = Some(rx);
        app.playlist_loading_id = Some(playlist_id);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = crate::app::playlists::fetch_playlist_chunks(
                    &token,
                    playlist_id,
                    tx,
                )
                .await
                {
                    log::error!(
                        "[Search] Failed to fetch playlist chunks: {}",
                        e
                    );
                }
            });
        });
    } else if !playlist.tracks.is_empty() {
        log::info!(
            "[Search] Playlist fully loaded with {} tracks",
            playlist.tracks.len()
        );
    }
}
