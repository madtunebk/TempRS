use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::helpers::{calculate_grid_layout, truncate_text};
use crate::utils::artwork::load_thumbnail_artwork;
use eframe::egui::{self, Color32, CornerRadius, Sense, Vec2};
use std::sync::mpsc::channel;

/// Render playlists search results grid with pagination
pub fn render_playlists_grid_paginated(
    app: &mut MusicPlayerApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
) {
    if app.content.search_results_playlists.is_empty() {
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
    let offset = app.content.search_page * app.content.search_page_size;
    let end =
        (offset + app.content.search_page_size).min(app.content.search_results_playlists.len());

    if offset >= app.content.search_results_playlists.len() {
        // Reset to first page if out of bounds
        return;
    }

    let page_playlists: Vec<_> = app.content.search_results_playlists[offset..end].to_vec();
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

    let (rect, response) = ui.allocate_exact_size(Vec2::new(size, size + 55.0), Sense::click());

    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), hover_bg);
    }

    let artwork_rect = egui::Rect::from_min_size(rect.min, Vec2::new(size, size));

    let artwork_url = playlist
        .artwork_url
        .as_ref()
        .map(|url| url.replace("-large.jpg", "-t500x500.jpg"))
        .unwrap_or_default();

    if !artwork_url.is_empty() {
        // Check memory cache first (fast path)
        if let Some(texture) = app.ui.thumb_cache.get(&artwork_url) {
            ui.painter().image(
                texture.id(),
                artwork_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
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

    // Add like button overlay (always show for search results)
    let is_liked = app.content.liked_playlist_ids.contains(&playlist.id);
    let heart_size = 32.0;
    let heart_pos = artwork_rect.min + egui::Vec2::new(4.0, 4.0);
    let heart_rect = egui::Rect::from_min_size(heart_pos, egui::Vec2::new(heart_size, heart_size));

    let heart_response = ui.interact(
        heart_rect,
        ui.id().with(("search_playlist_like", playlist.id)),
        Sense::click(),
    );

    // Heart button background (circle) with color based on state
    let bg_color = if heart_response.hovered() {
        Color32::from_rgba_premultiplied(255, 50, 50, 200) // Red on hover
    } else if is_liked {
        Color32::from_rgba_premultiplied(255, 85, 0, 200) // Orange when liked
    } else {
        Color32::from_rgba_premultiplied(80, 80, 80, 200) // Gray when not liked
    };

    ui.painter()
        .circle_filled(heart_rect.center(), heart_size / 2.0, bg_color);

    // Heart icon (filled if liked, broken if not)
    let heart_icon = if is_liked { "❤" } else { "💔" };
    ui.painter().text(
        heart_rect.center(),
        egui::Align2::CENTER_CENTER,
        heart_icon,
        egui::FontId::proportional(16.0),
        Color32::WHITE,
    );

    // Handle like/unlike click
    if heart_response.clicked() {
        log::info!(
            "[Search] Toggle like for playlist: {} ({})",
            playlist.title,
            playlist.id
        );
        app.toggle_playlist_like(playlist.id);
        return; // Don't trigger playlist load
    }

    // Show cursor hand on hover
    if heart_response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
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

    // Set selected playlist ID so like button appears in queue
    app.content.selected_playlist_id = Some(playlist.id);

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
            app.audio.playback_queue.load_tracks(preview_tracks);

            if let Some(first_track) = app.audio.playback_queue.current_track() {
                let track_id = first_track.id;
                app.play_track(track_id);
            }
        }
    } else if needs_full_fetch {
        // No preview tracks, clear queue and prepare for chunked loading
        log::info!("[Search] No preview tracks, clearing queue for fresh load");
        app.audio.playback_queue.load_tracks(Vec::new());
    }

    // If playlist is larger than preview, fetch full content in chunks
    if needs_full_fetch {
        log::info!(
            "[Search] Starting chunked fetch for {} total tracks",
            playlist.track_count
        );

        let playlist_id = playlist.id;
        let token = match app.content.app_state.get_token() {
            Some(t) => t,
            None => {
                log::error!("[Search] No token available for fetching full playlist");
                return;
            }
        };

        let (tx, rx) = channel();
        app.tasks.playlist_chunk_rx = Some(rx);
        app.content.playlist_loading_id = Some(playlist_id);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                if let Err(e) =
                    crate::app::playlists::fetch_playlist_chunks(&token, playlist_id, tx).await
                {
                    log::error!("[Search] Failed to fetch playlist chunks: {}", e);
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
