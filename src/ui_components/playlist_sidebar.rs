use eframe::egui;
use crate::app::player_app::MusicPlayerApp;
use crate::ui_components::colors::*;

pub fn render_playlist_tracks(app: &mut MusicPlayerApp, ui: &mut egui::Ui, ctx: &egui::Context) -> Option<usize> {
    let mut clicked_idx = None;
    
    // Get tracks in queue order (respects shuffle)
    let original_tracks = &app.audio.playback_queue.original_tracks;
    let queue_indices = &app.audio.playback_queue.current_queue;
    
    // Build playlist in queue order
    let playlist: Vec<_> = queue_indices
        .iter()
        .filter_map(|&idx| original_tracks.get(idx).cloned())
        .collect();
    
    // Clean up pending entries that are now in memory cache
    let cached_urls: Vec<String> = app.ui.thumb_cache.keys().cloned().collect();
    for url in cached_urls {
        app.ui.thumb_pending.remove(&url);
    }
    
    // Track if we need to auto-scroll
    let track_changed = app.audio.current_track_id != app.audio.last_track_id;
    
    // Get current queue position (simple index now since playlist is in queue order)
    let current_queue_idx = app.audio.playback_queue.current_index;
    
    ui.vertical(|ui| {
        ui.add_space(10.0);
        
        // Queue header with position counter
        ui.horizontal(|ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new("Queue")
                        .size(18.0)
                        .color(TEXT_PRIMARY)
                        .strong()
                )
            );
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Collapse button
                if ui.add_sized([30.0, 30.0], egui::Button::new("◀").fill(egui::Color32::from_rgb(35, 35, 40))).clicked() {
                    app.ui.queue_collapsed = true;
                }
                
                ui.add_space(5.0);
                
                // Like playlist button (only show if we have a selected playlist)
                if let Some(playlist_id) = app.content.selected_playlist_id {
                    let is_liked = app.content.liked_playlist_ids.contains(&playlist_id);
                    let heart_icon = if is_liked { "❤" } else { "♡" };
                    let heart_color = if is_liked { 
                        egui::Color32::from_rgb(255, 85, 0) // Orange when liked
                    } else { 
                        egui::Color32::from_rgb(160, 160, 160) // Gray when not liked
                    };
                    
                    let heart_btn = ui.add_sized(
                        [30.0, 30.0], 
                        egui::Button::new(heart_icon).fill(heart_color)
                    );
                    
                    if heart_btn.clicked() {
                        app.toggle_playlist_like(playlist_id);
                    }
                    
                    if heart_btn.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    
                    ui.add_space(5.0);
                }
                
                if let Some(pos) = current_queue_idx {
                    ui.label(
                        egui::RichText::new(format!("{} / {}", pos + 1, playlist.len()))
                            .size(13.0)
                            .color(TEXT_SECONDARY)
                    );
                }
            });
        });
        
        ui.add_space(10.0);
        
        ui.add(
            egui::Separator::default()
                .spacing(0.0)
        );
        
        ui.add_space(5.0);
        
        let total_tracks = playlist.len();
        let item_height = 75.0; // Height per track item
        
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, item_height, total_tracks, |ui, row_range| {
                for i in row_range {
                    if i >= playlist.len() {
                        break;
                    }
                    
                    let track = &playlist[i];
                    let is_current_track = app.audio.current_track_id == Some(track.id);
                    
                    let response = ui.add(
                        egui::Button::new("")
                            .fill(if is_current_track {
                                BG_HOVER // Orange-tinted highlight for playing track
                            } else {
                                BG_CARD
                            })
                            .corner_radius(6.0)
                            .min_size(egui::vec2(ui.available_width() - 10.0, 70.0))
                    );
                    
                    let rect = response.rect;
                    
                    // Auto-scroll to current track when it changes (smooth, centered)
                    if is_current_track && track_changed {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    
                    // Orange border for currently playing track
                    if is_current_track {
                        ui.painter().rect_stroke(
                            rect,
                            6.0,
                            egui::Stroke::new(2.0, ORANGE),
                            egui::epaint::StrokeKind::Outside,
                        );
                    }
                    
                    // Hover effect with subtle border
                    if response.hovered() && !is_current_track {
                        ui.painter().rect_filled(
                            rect,
                            6.0,
                            BG_BUTTON,
                        );
                        ui.painter().rect_stroke(
                            rect,
                            6.0,
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 85, 0)),
                            egui::epaint::StrokeKind::Outside,
                        );
                    }
                    
                    // Playing indicator with glow
                    if is_current_track {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                rect.min,
                                egui::vec2(4.0, rect.height()),
                            ),
                            6.0,
                            egui::Color32::from_rgb(255, 85, 0),
                        );
                    }
                    
                    // Artwork thumbnail with shadow
                    let artwork_size = 55.0;
                    let artwork_rect = egui::Rect::from_min_size(
                        rect.min + egui::vec2(12.0, 7.5),
                        egui::vec2(artwork_size, artwork_size),
                    );
                    
                    // Ambient glow effect around artwork (only for currently playing track)
                    if app.audio.current_track_id == Some(track.id) {
                        // More visible glow - 2 layers
                        for i in 0..2 {
                            let expansion = (i + 1) as f32 * 2.5;
                            let alpha = 50 - (i * 20) as u8;
                            let glow_rect = artwork_rect.expand(expansion);
                            ui.painter().rect_filled(
                                glow_rect,
                                6.0,
                                egui::Color32::from_rgba_premultiplied(255, 85, 0, alpha),
                            );
                        }
                    }
                    
                    // Try to load and display artwork thumbnail
                    if let Some(artwork_url) = &track.artwork_url {
                        // Normalize URL to high quality format
                        let hq_url = artwork_url.replace("-large.jpg", "-t500x500.jpg");
                        
                        // Check if we have this thumbnail cached in memory
                        if let Some(texture) = app.ui.thumb_cache.get(&hq_url) {
                            // Draw cached thumbnail
                            let mut mesh = egui::Mesh::with_texture(texture.id());
                            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                            mesh.add_rect_with_uv(
                                artwork_rect,
                                uv,
                                egui::Color32::WHITE,
                            );
                            ui.painter().add(egui::Shape::mesh(mesh));
                        } else {
                            // Not in memory cache, check disk cache using track ID (FAST PATH)
                            if let Some(cached_data) = crate::utils::cache::load_artwork_cache(track.id) {
                                // Load from disk cache into memory
                                if let Ok(decoded) = image::load_from_memory(&cached_data) {
                                    let rgba = decoded.to_rgba8();
                                    let (w, h) = rgba.dimensions();
                                    let img = egui::ColorImage::from_rgba_unmultiplied(
                                        [w as usize, h as usize],
                                        &rgba,
                                    );
                                    let texture = ctx.load_texture(&hq_url, img, egui::TextureOptions::LINEAR);
                                    app.ui.thumb_cache.insert(hq_url.clone(), texture.clone());
                                    
                                    // Clear pending flag since we loaded it
                                    app.ui.thumb_pending.remove(&hq_url);
                                    
                                    // Draw it now
                                    let mut mesh = egui::Mesh::with_texture(texture.id());
                                    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                                    mesh.add_rect_with_uv(
                                        artwork_rect,
                                        uv,
                                        egui::Color32::WHITE,
                                    );
                                    ui.painter().add(egui::Shape::mesh(mesh));
                                } else {
                                    // Cached data corrupted, show no_artwork placeholder
                                    if let Some(no_artwork) = &app.ui.no_artwork_texture {
                                        let mut mesh = egui::Mesh::with_texture(no_artwork.id());
                                        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                                        mesh.add_rect_with_uv(
                                            artwork_rect,
                                            uv,
                                            egui::Color32::WHITE,
                                        );
                                        ui.painter().add(egui::Shape::mesh(mesh));
                                    } else {
                                        ui.painter().rect_filled(
                                            artwork_rect,
                                            4.0,
                                            egui::Color32::from_rgb(60, 60, 60),
                                        );
                                    }
                                }
                            } else if !app.ui.thumb_pending.get(&hq_url).unwrap_or(&false) {
                                // Not in disk cache and not loading, start fetching (SLOW PATH)
                                app.ui.thumb_pending.insert(hq_url.clone(), true);
                                request_thumb_fetch(ctx, track.id, &hq_url);
                                
                                // Draw no_artwork placeholder while loading
                                if let Some(no_artwork) = &app.ui.no_artwork_texture {
                                    let mut mesh = egui::Mesh::with_texture(no_artwork.id());
                                    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                                    mesh.add_rect_with_uv(
                                        artwork_rect,
                                        uv,
                                        egui::Color32::WHITE,
                                    );
                                    ui.painter().add(egui::Shape::mesh(mesh));
                                } else {
                                    ui.painter().rect_filled(
                                        artwork_rect,
                                        4.0,
                                        egui::Color32::from_rgb(60, 60, 60),
                                    );
                                }
                            } else {
                                // Loading in progress, show no_artwork placeholder
                                if let Some(no_artwork) = &app.ui.no_artwork_texture {
                                    let mut mesh = egui::Mesh::with_texture(no_artwork.id());
                                    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                                    mesh.add_rect_with_uv(
                                        artwork_rect,
                                        uv,
                                        egui::Color32::WHITE,
                                    );
                                    ui.painter().add(egui::Shape::mesh(mesh));
                                } else {
                                    ui.painter().rect_filled(
                                        artwork_rect,
                                        4.0,
                                        egui::Color32::from_rgb(60, 60, 60),
                                    );
                                }
                            }
                        }
                    } else {
                        // No artwork URL, use no_artwork placeholder
                        if let Some(no_artwork) = &app.ui.no_artwork_texture {
                            let mut mesh = egui::Mesh::with_texture(no_artwork.id());
                            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                            mesh.add_rect_with_uv(
                                artwork_rect,
                                uv,
                                egui::Color32::WHITE,
                            );
                            ui.painter().add(egui::Shape::mesh(mesh));
                        } else {
                            // Fallback solid color
                            ui.painter().rect_filled(
                                artwork_rect,
                                4.0,
                                egui::Color32::from_rgb(60, 60, 60),
                            );
                        }
                    }
                    
                    // Artwork border with subtle shadow
                    ui.painter().rect_stroke(
                        artwork_rect,
                        6.0,
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(60, 60, 65)),
                        egui::epaint::StrokeKind::Outside,
                    );
                    
                    // Track info
                    let text_pos = artwork_rect.max + egui::vec2(15.0, -artwork_size + 10.0);
                    
                    // Track title (truncated if too long)
                    let title = if track.title.chars().count() > 35 {
                        let truncated: String = track.title.chars().take(35).collect();
                        format!("{}...", truncated)
                    } else {
                        track.title.clone()
                    };
                    
                    ui.painter().text(
                        text_pos,
                        egui::Align2::LEFT_TOP,
                        &title,
                        egui::FontId::proportional(13.0),
                        egui::Color32::from_rgb(240, 240, 240),
                    );
                    
                    // Artist name from user
                    ui.painter().text(
                        text_pos + egui::vec2(0.0, 20.0),
                        egui::Align2::LEFT_TOP,
                        &track.user.username,
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_rgb(150, 150, 150),
                    );
                    
                    // Duration on the right (convert ms to mm:ss)
                    let duration_secs = track.duration / 1000;
                    let duration_str = format!("{}:{:02}", duration_secs / 60, duration_secs % 60);
                    ui.painter().text(
                        egui::pos2(rect.max.x - 10.0, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        &duration_str,
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_rgb(120, 120, 120),
                    );
                    
                    if response.clicked() {
                        clicked_idx = Some(i);
                    }
                    
                    ui.add_space(5.0);
                }
                
                // Add bottom padding so last track isn't cut off by player controls
                // Footer is 53px + need extra space for comfortable viewing
                ui.add_space(80.0);
            });
    });
    
    // Update last_track_id to prevent repeated scrolling
    if track_changed {
        app.audio.last_track_id = app.audio.current_track_id;
    }
    
    clicked_idx
}

/// Request thumbnail fetch in background (optimized, no concurrent limit)
fn request_thumb_fetch(ctx: &egui::Context, track_id: u64, url: &str) {
    if url.is_empty() {
        return;
    }
    
    let url_clone = url.to_string();
    let ctx_clone = ctx.clone();
    
    // Spawn single thread per image for better parallelism (same as search)
    crate::utils::async_helper::spawn_fire_and_forget(move || {
        Box::pin(async move {
            let client = crate::utils::http::client();

            match client.get(&url_clone).send().await {
                Ok(resp) => {
                    if let Ok(bytes) = resp.bytes().await {
                        // Save to shared artwork cache using track ID
                        if let Err(e) = crate::utils::cache::save_artwork_cache(track_id, &bytes, false) {
                            log::warn!("[Playlist Sidebar] Failed to cache {}: {}", url_clone, e);
                        }
                        // Trigger repaint so UI loads from cache
                        ctx_clone.request_repaint();
                    }
                }
                Err(e) => {
                    log::warn!("[Playlist Sidebar] Download failed {}: {}", url_clone, e);
                    // Save placeholder to prevent retry loops
                    let _ = crate::utils::cache::save_artwork_cache(track_id, &[], true);
                }
            }

            Ok(())
        })
    });
}
