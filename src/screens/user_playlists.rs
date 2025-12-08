use eframe::egui;
use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::helpers::calculate_grid_layout;
use crate::ui_components::colors::*;

pub fn render_user_playlists_view(app: &mut MusicPlayerApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    // Fetch playlists on first visit
    if !app.content.playlists_initial_fetch_done && !app.content.playlists_loading {
        app.fetch_playlists();
        app.content.playlists_initial_fetch_done = true;
    }
    
    // Check for background fetch completion
    app.check_playlists_updates();
    
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(20.0);
            
            // Title with playlist count
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("📚 My Playlists")
                        .size(24.0)
                        .color(egui::Color32::WHITE)
                        .strong()
                );
                
                if !app.content.playlists.is_empty() {
                    ui.add_space(15.0);
                    ui.label(
                        egui::RichText::new(format!("({} playlists)", app.content.playlists.len()))
                            .size(16.0)
                            .color(egui::Color32::GRAY)
                    );
                }
            });
            
            ui.add_space(20.0);
            
            // Show playlists
            render_playlists(app, ui, ctx);
        });
}

fn render_playlists(app: &mut MusicPlayerApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    if app.content.playlists_loading {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.spinner();
            ui.add_space(10.0);
            ui.label("Loading playlists...");
        });
        return;
    }

    if app.content.playlists.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.label(
                egui::RichText::new("📚")
                    .size(64.0)
                    .color(egui::Color32::GRAY)
            );
            ui.add_space(15.0);
            ui.label(
                egui::RichText::new("No playlists yet")
                    .size(20.0)
                    .color(egui::Color32::GRAY)
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Create or like some playlists!")
                    .size(14.0)
                    .color(egui::Color32::DARK_GRAY)
            );
        });
        return;
    }

    // Apply filter
    let mut filtered_playlists = app.content.playlists.clone();
    let filter_text = app.content.playlists_search_filter.to_lowercase();
    if !filter_text.is_empty() {
        filtered_playlists.retain(|playlist| {
            playlist.title.to_lowercase().contains(&filter_text) ||
            playlist.user.username.to_lowercase().contains(&filter_text)
        });
    }

    // Apply sorting
    match app.content.playlists_sort_order {
        crate::app::player_app::PlaylistsSortOrder::RecentFirst => {
            // Already in order from API
        }
        crate::app::player_app::PlaylistsSortOrder::NameAZ => {
            filtered_playlists.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        }
        crate::app::player_app::PlaylistsSortOrder::TrackCount => {
            filtered_playlists.sort_by(|a, b| b.track_count.cmp(&a.track_count));
        }
    }

    // Calculate pagination
    let total_playlists = filtered_playlists.len();
    let start_idx = app.content.playlists_page * app.content.playlists_page_size;
    let end_idx = (start_idx + app.content.playlists_page_size).min(total_playlists);
    let page_playlists: Vec<_> = filtered_playlists[start_idx..end_idx].to_vec();

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
            egui::TextEdit::singleline(&mut app.content.playlists_search_filter)
                .hint_text("Filter by playlist name or creator...")
                .desired_width(300.0)
        );

        // Reset to page 0 when filter changes
        if search_response.changed() {
            app.content.playlists_page = 0;
        }

        // Clear button
        if !app.content.playlists_search_filter.is_empty() {
            ui.add_space(5.0);
            if ui.button("✖").clicked() {
                app.content.playlists_search_filter.clear();
                app.content.playlists_page = 0;
            }
        }

        ui.add_space(20.0);

        // Sort dropdown
        ui.label(egui::RichText::new("Sort:").size(14.0).color(egui::Color32::GRAY));
        ui.add_space(5.0);

        egui::ComboBox::from_id_source("playlists_sort")
            .selected_text(match app.content.playlists_sort_order {
                crate::app::player_app::PlaylistsSortOrder::RecentFirst => "Recent First",
                crate::app::player_app::PlaylistsSortOrder::NameAZ => "Name (A-Z)",
                crate::app::player_app::PlaylistsSortOrder::TrackCount => "Track Count",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.content.playlists_sort_order, crate::app::player_app::PlaylistsSortOrder::RecentFirst, "Recent First");
                ui.selectable_value(&mut app.content.playlists_sort_order, crate::app::player_app::PlaylistsSortOrder::NameAZ, "Name (A-Z)");
                ui.selectable_value(&mut app.content.playlists_sort_order, crate::app::player_app::PlaylistsSortOrder::TrackCount, "Track Count");
            });
    });

    ui.add_space(15.0);
    
    // Render playlists in grid
    render_playlists_grid(app, ui, ctx, &page_playlists);
    
    ui.add_space(30.0);
    
    // Pagination controls (centered)
    crate::ui_components::helpers::render_pagination_controls(
        ui,
        &mut app.content.playlists_page,
        total_playlists,
        app.content.playlists_page_size,
    );
    
    ui.add_space(20.0);
}

fn render_playlists_grid(app: &mut MusicPlayerApp, ui: &mut egui::Ui, _ctx: &egui::Context, playlists: &[crate::models::playlist::Playlist]) {
    let card_size = 220.0;
    let spacing = 15.0;
    
    let (cards_per_row, padding) = calculate_grid_layout(ui.available_width(), card_size, spacing);
    
    for row_playlists in playlists.chunks(cards_per_row) {
        ui.horizontal(|ui| {
            ui.add_space(padding);
            for playlist in row_playlists {
                render_playlist_card(app, ui, playlist, card_size);
                ui.add_space(spacing);
            }
        });
        ui.add_space(spacing);
    }
}

/// Render a single playlist card with artwork
/// Standardized: 1:1 aspect ratio, 8px padding, smooth hover transitions
fn render_playlist_card(
    app: &mut MusicPlayerApp,
    ui: &mut egui::Ui,
    playlist: &crate::models::playlist::Playlist,
    card_size: f32,
) {
    use eframe::egui::{Vec2, Sense, Color32};
    
    // Card standardization: fixed padding and metadata height
    let card_padding = 8.0;
    let metadata_height = 60.0;
    let full_height = card_size + metadata_height;
    
    let (rect, response) = ui.allocate_exact_size(Vec2::new(card_size, full_height), Sense::click());
    
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
    
    // Load playlist artwork
    let artwork_url = playlist.artwork_url.as_deref();
    let cache_key = format!("playlist:{}", playlist.id);
    let ctx = ui.ctx().clone();
    
    if let Some(texture) = crate::utils::artwork::load_track_artwork(
        app,
        &ctx,
        playlist.id,
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
            crate::utils::artwork::load_thumbnail_artwork(app, &ctx, playlist.id, url_high, false);
        }
        
        // Enhanced skeleton placeholder with icon overlay
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
        
        // Playlist icon overlay
        ui.painter().text(
            artwork_rect.center(),
            egui::Align2::CENTER_CENTER,
            "≡",
            egui::FontId::proportional(48.0),
            Color32::from_rgb(100, 100, 100),
        );
        
        ui.ctx().request_repaint();
    }
    
    // Unlike/like button (heart icon overlay on artwork) - only for non-user playlists
    let is_user_playlist = app.content.user_created_playlist_ids.contains(&playlist.id);
    
    if !is_user_playlist {
        let is_liked = app.content.liked_playlist_ids.contains(&playlist.id);
        let heart_size = 32.0;
        let heart_pos = artwork_rect.min + egui::Vec2::new(4.0, 4.0);
        let heart_rect = egui::Rect::from_min_size(heart_pos, egui::Vec2::new(heart_size, heart_size));
        
        let heart_response = ui.interact(
            heart_rect,
            ui.id().with(("unlike_playlist", playlist.id)),
            egui::Sense::click()
        );
        
        // Heart button background (circle) with orange color for liked playlists
        let bg_color = if heart_response.hovered() {
            egui::Color32::from_rgba_premultiplied(255, 50, 50, 200)  // Red on hover
        } else if is_liked {
            egui::Color32::from_rgba_premultiplied(255, 85, 0, 200)  // Orange when liked
        } else {
            egui::Color32::from_rgba_premultiplied(0, 0, 0, 140)  // Dark when not liked
        };
        
        ui.painter().circle_filled(
            heart_rect.center(),
            heart_size / 2.0,
            bg_color
        );
        
        // Heart icon (filled if liked, broken if not)
        let heart_icon = if is_liked { "❤" } else { "💔" };
        ui.painter().text(
            heart_rect.center(),
            egui::Align2::CENTER_CENTER,
            heart_icon,
            egui::FontId::proportional(16.0),
            egui::Color32::WHITE
        );
        
        // Handle unlike/like click
        if heart_response.clicked() {
            log::info!("[Playlists] Toggle like for playlist: {} ({})", playlist.title, playlist.id);
            app.toggle_playlist_like(playlist.id);
            return; // Don't trigger playlist load
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
    
    // Metadata section with consistent padding
    let metadata_y = rect.min.y + card_size + card_padding;
    
    // Title text (truncated to fit, with consistent padding)
    let title_pos = egui::pos2(rect.min.x + card_padding, metadata_y);
    let title_text = crate::ui_components::helpers::truncate_text(&playlist.title, 25);
    ui.painter().text(
        title_pos,
        egui::Align2::LEFT_TOP,
        title_text,
        egui::FontId::proportional(13.0),
        Color32::from_rgb(240, 240, 240),
    );
    
    // Track count (truncated to fit, with consistent padding and spacing)
    let count_pos = egui::pos2(rect.min.x + card_padding, metadata_y + 20.0);
    let count_text = format!("{} tracks", playlist.track_count);
    ui.painter().text(
        count_pos,
        egui::Align2::LEFT_TOP,
        count_text,
        egui::FontId::proportional(11.0),
        Color32::from_rgb(160, 160, 160),
    );
    
    // Handle click - load playlist tracks into queue and play
    if response.clicked() {
        if !playlist.tracks.is_empty() {
            // Playlist already has tracks - load directly into queue
            log::info!("[Playlists] Loading playlist '{}' with {} tracks", playlist.title, playlist.tracks.len());
            app.audio.playback_queue.load_tracks(playlist.tracks.clone());
            
            // Start playing first track
            if let Some(first_track) = app.audio.playback_queue.current_track() {
                log::info!("[Playlists] Starting playback: {}", first_track.title);
                app.play_track(first_track.id);
            }
        } else {
            // Playlist has no tracks - need to fetch full details
            log::info!("[Playlists] Clicked liked playlist '{}' - fetching full details (ID: {})", playlist.title, playlist.id);
            app.content.selected_playlist_id = Some(playlist.id);
            app.content.playlist_loading_id = Some(playlist.id);
            
            // Fetch playlist with tracks
            if let Some(oauth) = &app.auth.oauth_manager {
                if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                    let token = token_data.access_token.clone();
                    let playlist_id = playlist.id;
                    let (tx, rx) = std::sync::mpsc::channel();
                    app.tasks.playlist_rx = Some(rx);
                    
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            match crate::api::fetch_playlist_by_id(&token, playlist_id).await {
                                Ok(full_playlist) => {
                                    log::info!("[Playlists] Successfully fetched playlist details");
                                    let _ = tx.send(full_playlist);
                                }
                                Err(e) => {
                                    log::error!("[Playlists] Failed to fetch playlist details: {}", e);
                                }
                            }
                        });
                    });
                }
            }
        }
    }
}


