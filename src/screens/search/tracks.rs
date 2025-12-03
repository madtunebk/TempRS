use eframe::egui::{self, Vec2, Sense, Color32, CornerRadius};
use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::helpers::{truncate_text, calculate_grid_layout};
use crate::utils::artwork::load_thumbnail_artwork;

/// Render tracks search results grid with pagination
pub fn render_tracks_grid_paginated(app: &mut MusicPlayerApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    if app.search_results_tracks.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.label(
                egui::RichText::new("No tracks found")
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
    let end = (offset + app.search_page_size).min(app.search_results_tracks.len());
    
    if offset >= app.search_results_tracks.len() {
        // Reset to first page if out of bounds
        return;
    }
    
    let page_tracks: Vec<_> = app.search_results_tracks[offset..end].to_vec();
    let (items_per_row, padding) = calculate_grid_layout(ui.available_width(), 220.0, 15.0);

    ui.add_space(10.0);

    for chunk in page_tracks.chunks(items_per_row) {
        ui.horizontal(|ui| {
            ui.add_space(padding);
            for track in chunk {
                render_track_item(app, ui, ctx, track, 220.0);
                ui.add_space(15.0);
            }
        });
        ui.add_space(15.0);
    }
}

fn render_track_item(
    app: &mut MusicPlayerApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    track: &crate::app::playlists::Track,
    size: f32,
) {
    let hover_bg = Color32::from_rgb(40, 40, 45);

    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(size, size + 55.0),
        Sense::click(),
    );

    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), hover_bg);
    }

    let artwork_rect =
        egui::Rect::from_min_size(rect.min, Vec2::new(size, size));

    let artwork_url = track
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
            load_thumbnail_artwork(app, ctx, track.id, artwork_url.clone(), false);
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
        play_track(app, track);
    }

    let text_rect = egui::Rect::from_min_size(
        artwork_rect.min + Vec2::new(0.0, size + 5.0),
        Vec2::new(size, 50.0),
    );

    ui.painter().text(
        text_rect.min + Vec2::new(5.0, 0.0),
        egui::Align2::LEFT_TOP,
        truncate_text(&track.title, 25),
        egui::FontId::proportional(13.0),
        Color32::WHITE,
    );

    ui.painter().text(
        text_rect.min + Vec2::new(5.0, 18.0),
        egui::Align2::LEFT_TOP,
        truncate_text(&track.user.username, 28),
        egui::FontId::proportional(11.0),
        Color32::GRAY,
    );
}

fn play_track(app: &mut MusicPlayerApp, track: &crate::app::playlists::Track) {
    log::info!("[Search] Playing track: {}", track.title);
    app.playback_queue.load_tracks(vec![track.clone()]);
    if let Some(current_track) = app.playback_queue.current_track() {
        let track_id = current_track.id;
        app.play_track(track_id);
    }
}
