use eframe::egui;
use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::helpers::calculate_grid_layout;
use crate::ui_components::colors::*;

pub fn render_likes_view(app: &mut MusicPlayerApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    // Fetch likes on first visit
    if !app.content.likes_initial_fetch_done && !app.content.likes_loading {
        app.fetch_likes();
        app.content.likes_initial_fetch_done = true;
    }
    
    // Check for background fetch completion
    app.check_likes_updates();
    
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(20.0);
            
            // Title with track count
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("💜 Liked Tracks")
                        .size(24.0)
                        .color(egui::Color32::WHITE)
                        .strong()
                );
                
                let total = app.content.likes_tracks.len() + app.content.user_tracks.len();
                if total > 0 {
                    ui.add_space(15.0);
                    ui.label(
                        egui::RichText::new(format!("({} tracks)", total))
                            .size(16.0)
                            .color(egui::Color32::GRAY)
                    );
                }
            });
            
            ui.add_space(20.0);
            
            // Show tracks
            render_liked_tracks(app, ui, ctx);
        });
}

fn render_liked_tracks(app: &mut MusicPlayerApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    if app.content.likes_loading {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.spinner();
            ui.add_space(10.0);
            ui.label("Loading tracks...");
        });
        return;
    }

    // Combine liked tracks + user uploaded tracks
    let mut all_tracks_with_badges: Vec<(crate::models::track::Track, &str)> = Vec::new();

    // Add liked tracks with 💜 badge
    for track in &app.content.likes_tracks {
        all_tracks_with_badges.push((track.clone(), "💜"));
    }

    // Add user uploaded tracks with 🎤 badge
    for track in &app.content.user_tracks {
        all_tracks_with_badges.push((track.clone(), "🎤"));
    }

    if all_tracks_with_badges.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.label(
                egui::RichText::new("💜")
                    .size(64.0)
                    .color(egui::Color32::GRAY)
            );
            ui.add_space(15.0);
            ui.label(
                egui::RichText::new("No tracks yet")
                    .size(20.0)
                    .color(egui::Color32::GRAY)
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Start liking or uploading tracks!")
                    .size(14.0)
                    .color(egui::Color32::DARK_GRAY)
            );
        });
        return;
    }

    // Apply filter
    let filter_text = app.content.likes_search_filter.to_lowercase();
    if !filter_text.is_empty() {
        all_tracks_with_badges.retain(|(track, _)| {
            track.title.to_lowercase().contains(&filter_text) ||
            track.user.username.to_lowercase().contains(&filter_text) ||
            track.genre.as_ref().is_some_and(|g| g.to_lowercase().contains(&filter_text))
        });
    }

    // Apply sorting
    match app.content.likes_sort_order {
        crate::app::player_app::LikesSortOrder::RecentFirst => {
            // Already in order from API
        }
        crate::app::player_app::LikesSortOrder::TitleAZ => {
            all_tracks_with_badges.sort_by(|a, b| a.0.title.to_lowercase().cmp(&b.0.title.to_lowercase()));
        }
        crate::app::player_app::LikesSortOrder::ArtistAZ => {
            all_tracks_with_badges.sort_by(|a, b| a.0.user.username.to_lowercase().cmp(&b.0.user.username.to_lowercase()));
        }
    }

    // Calculate pagination
    let total_tracks = all_tracks_with_badges.len();
    let start_idx = app.content.likes_page * app.content.likes_page_size;
    let end_idx = (start_idx + app.content.likes_page_size).min(total_tracks);
    let page_tracks: Vec<_> = all_tracks_with_badges[start_idx..end_idx].to_vec();

    // Calculate padding for alignment with grid
    let (_, grid_padding) = calculate_grid_layout(ui.available_width(), 220.0, 15.0);

    // Filter/Sort bar
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        ui.add_space(grid_padding);

        // Search filter
        ui.label(egui::RichText::new("🔍").size(18.0));
        ui.add_space(8.0);

        let search_response = ui.add_sized(
            egui::vec2(300.0, 32.0),
            egui::TextEdit::singleline(&mut app.content.likes_search_filter)
                .hint_text("Filter by title, artist, or genre...")
                .desired_width(300.0)
        );

        // Reset to page 0 when filter changes
        if search_response.changed() {
            app.content.likes_page = 0;
        }

        // Clear button
        if !app.content.likes_search_filter.is_empty() {
            ui.add_space(5.0);
            if ui.button("✖").clicked() {
                app.content.likes_search_filter.clear();
                app.content.likes_page = 0;
            }
        }

        ui.add_space(20.0);

        // Sort dropdown
        ui.label(egui::RichText::new("Sort:").size(14.0).color(egui::Color32::GRAY));
        ui.add_space(5.0);

        egui::ComboBox::from_id_salt("likes_sort")
            .selected_text(match app.content.likes_sort_order {
                crate::app::player_app::LikesSortOrder::RecentFirst => "Recent First",
                crate::app::player_app::LikesSortOrder::TitleAZ => "Title (A-Z)",
                crate::app::player_app::LikesSortOrder::ArtistAZ => "Artist (A-Z)",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.content.likes_sort_order, crate::app::player_app::LikesSortOrder::RecentFirst, "Recent First");
                ui.selectable_value(&mut app.content.likes_sort_order, crate::app::player_app::LikesSortOrder::TitleAZ, "Title (A-Z)");
                ui.selectable_value(&mut app.content.likes_sort_order, crate::app::player_app::LikesSortOrder::ArtistAZ, "Artist (A-Z)");
            });
    });

    ui.add_space(15.0);
    
    // Render tracks in grid with badges
    render_tracks_grid_with_badges(app, ui, ctx, &page_tracks);
    
    ui.add_space(30.0);
    
    // Pagination controls (centered)
    crate::ui_components::helpers::render_pagination_controls(
        ui,
        &mut app.content.likes_page,
        total_tracks,
        app.content.likes_page_size,
    );
    
    ui.add_space(20.0);
}

fn render_tracks_grid_with_badges(app: &mut MusicPlayerApp, ui: &mut egui::Ui, _ctx: &egui::Context, tracks_with_badges: &[(crate::models::track::Track, &str)]) {
    let card_size = 220.0;
    let spacing = 15.0;
    
    let (cards_per_row, padding) = calculate_grid_layout(ui.available_width(), card_size, spacing);
    
    for row_tracks in tracks_with_badges.chunks(cards_per_row) {
        ui.horizontal(|ui| {
            ui.add_space(padding);
            for (track, badge) in row_tracks {
                // Render track card
                let (clicked, _shift_clicked, unlike_clicked) = render_track_card_with_badge(
                    app,
                    ui,
                    track,
                    card_size,
                    badge,
                );
                
                if clicked {
                    app.audio.playback_queue.load_tracks(vec![track.clone()]);
                    app.play_track(track.id);
                }
                
                // Handle unlike click
                if unlike_clicked {
                    app.toggle_like(track.id);
                }
                
                ui.add_space(spacing);
            }
        });
        ui.add_space(spacing);
    }
}

/// Render a track card with a corner badge (💜 for liked, 🎤 for uploaded)
/// Standardized: 1:1 aspect ratio, 8px padding, smooth hover transitions
fn render_track_card_with_badge(
    app: &mut crate::app::player_app::MusicPlayerApp,
    ui: &mut egui::Ui,
    track: &crate::models::track::Track,
    card_size: f32,
    badge: &str,
) -> (bool, bool, bool) {  // Returns (clicked, shift_clicked, unlike_clicked)
    use eframe::egui::{Vec2, Sense, Color32};
    
    // Card standardization: fixed padding and metadata height
    let card_padding = 8.0;
    let metadata_height = 60.0;
    let full_height = card_size + metadata_height;
    
    let (rect, response) = ui.allocate_exact_size(Vec2::new(card_size, full_height), Sense::click());
    
    let clicked = response.clicked();
    let shift_clicked = response.clicked() && ui.input(|i| i.modifiers.shift);
    
    // Enhanced hover effect with subtle background and border
    if response.hovered() {
        // Subtle background fill
        ui.painter().rect_filled(
            rect,
            8.0,
            BG_HOVER,
        );
        // Orange accent border on hover
        ui.painter().rect_stroke(
            rect,
            8.0,
            egui::Stroke::new(1.5, Color32::from_rgb(255, 85, 0)),
            egui::epaint::StrokeKind::Outside,
        );
    }
    
    // Artwork with fixed 1:1 aspect ratio and consistent padding
    let artwork_rect = egui::Rect::from_min_size(
        rect.min + Vec2::new(card_padding, card_padding),
        Vec2::new(card_size - card_padding * 2.0, card_size - card_padding * 2.0),
    );
    
    // Use centralized artwork loader
    let artwork_url = track.artwork_url.as_deref();
    let cache_key = format!("track:{}", track.id);
    let ctx = ui.ctx().clone();
    
    if let Some(texture) = crate::utils::artwork::load_track_artwork(
        app,
        &ctx,
        track.id,
        artwork_url,
        &cache_key,
    ) {
        ui.painter().image(
            texture.id(),
            artwork_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        // Not in cache - trigger download if we have URL
        if let Some(url) = artwork_url {
            let url_high = url.replace("-large.jpg", "-t500x500.jpg");
            crate::utils::artwork::load_thumbnail_artwork(app, &ctx, track.id, url_high, false);
            
            // Enhanced skeleton placeholder with smooth pulse animation
            let pulse_speed = 1.8;
            let time = ui.input(|i| i.time) as f32;
            let pulse = ((time * pulse_speed).sin() * 0.5 + 0.5) * 12.0;
            
            let base_color = 55;
            let animated_color = (base_color as f32 + pulse) as u8;
            
            ui.painter().rect_filled(
                artwork_rect,
                6.0,
                Color32::from_rgb(animated_color, animated_color, animated_color + 5),
            );
            
            ui.ctx().request_repaint();
        } else {
            // No artwork URL - show no_artwork.png placeholder
            if let Some(no_artwork) = &app.ui.no_artwork_texture {
                ui.painter().image(
                    no_artwork.id(),
                    artwork_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                // Fallback gray box if no_artwork texture not loaded yet
                ui.painter().rect_filled(
                    artwork_rect,
                    6.0,
                    Color32::from_rgb(55, 55, 60),
                );
            }
        }
    }
    
    // Hover overlay with play button icon
    if response.hovered() {
        // Semi-transparent black overlay
        ui.painter().rect_filled(
            artwork_rect,
            6.0,
            OVERLAY_DARK,
        );
        
        // Play button circle in center
        let center = artwork_rect.center();
        let play_btn_radius = 28.0;
        
        // Orange circle background
        ui.painter().circle_filled(
            center,
            play_btn_radius,
            Color32::from_rgb(255, 85, 0),
        );
        
        // Play triangle icon
        let triangle_size = 16.0;
        let triangle_offset_x = 3.0;
        ui.painter().text(
            center + egui::Vec2::new(triangle_offset_x, 0.0),
            egui::Align2::CENTER_CENTER,
            "▶",
            egui::FontId::proportional(triangle_size),
            Color32::WHITE,
        );
    }
    
    // Unlike button in top-left corner (only for liked tracks)
    let mut unlike_clicked = false;
    if badge == "💜" {
        let heart_size = 32.0;
        let heart_pos = artwork_rect.min + Vec2::new(4.0, 4.0);
        let heart_rect = egui::Rect::from_min_size(heart_pos, Vec2::new(heart_size, heart_size));
        
        let heart_response = ui.interact(heart_rect, ui.id().with(("unlike", track.id)), Sense::click());
        
        // Background circle with orange color for liked tracks
        let bg_color = if heart_response.hovered() {
            Color32::from_rgba_premultiplied(255, 50, 50, 200)  // Bright red on hover
        } else {
            Color32::from_rgba_premultiplied(255, 85, 0, 200)  // Orange background for liked
        };
        
        ui.painter().circle_filled(
            heart_rect.center(),
            heart_size / 2.0,
            bg_color,
        );
        
        // Heart icon
        let heart_icon = if app.content.liked_track_ids.contains(&track.id) {
            "❤"  // Filled heart (liked)
        } else {
            "💔"  // Broken heart (unliked)
        };
        
        ui.painter().text(
            heart_rect.center(),
            egui::Align2::CENTER_CENTER,
            heart_icon,
            egui::FontId::proportional(18.0),
            Color32::WHITE,
        );
        
        if heart_response.clicked() {
            unlike_clicked = true;
        }
        
        // Show cursor pointer on hover
        if heart_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    } else if badge == "🎤" {
        // Show microphone badge for uploaded tracks in top-left corner
        let badge_size = 32.0;
        let badge_pos = artwork_rect.min + Vec2::new(4.0, 4.0);
        let badge_rect = egui::Rect::from_min_size(badge_pos, Vec2::new(badge_size, badge_size));
        
        ui.painter().circle_filled(
            badge_rect.center(),
            badge_size / 2.0,
            Color32::from_rgba_premultiplied(0, 0, 0, 150),
        );
        
        ui.painter().text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            badge,
            egui::FontId::proportional(18.0),
            Color32::WHITE,
        );
    }
    
    // Metadata section with consistent padding
    let metadata_y = rect.min.y + card_size + card_padding;
    
    // Track title (truncated to fit, with consistent padding)
    let title_pos = egui::pos2(rect.min.x + card_padding, metadata_y);
    let title_text = crate::ui_components::helpers::truncate_text(&track.title, 25);
    ui.painter().text(
        title_pos,
        egui::Align2::LEFT_TOP,
        title_text,
        egui::FontId::proportional(13.0),
        Color32::from_rgb(240, 240, 240),
    );
    
    // Artist name (truncated to fit, with consistent padding and spacing)
    let artist_pos = egui::pos2(rect.min.x + card_padding, metadata_y + 20.0);
    let artist_text = crate::ui_components::helpers::truncate_text(&track.user.username, 28);
    ui.painter().text(
        artist_pos,
        egui::Align2::LEFT_TOP,
        artist_text,
        egui::FontId::proportional(11.0),
        Color32::from_rgb(160, 160, 160),
    );
    
    (clicked, shift_clicked, unlike_clicked)
}


