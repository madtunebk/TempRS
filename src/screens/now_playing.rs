use eframe::egui;
use crate::app::player_app::MusicPlayerApp;
use crate::utils::{ShaderCallback, MultiPassCallback};

/// Now Playing screen - Shows current track with large artwork, shader background, and audio-reactive glow
pub fn render_now_playing_view(app: &mut MusicPlayerApp, ui: &mut egui::Ui, _ctx: &egui::Context) {
    // Show error message if playback failed
    if let Some(error_msg) = &app.ui.last_playback_error {
        render_error_state(ui, error_msg);
        return;
    }
    
    // Show placeholder if no track is playing
    if app.audio.current_track_id.is_none() {
        render_empty_state(ui);
        return;
    }
    
    // Get current track from queue
    if let Some(current_track) = app.audio.playback_queue.current_track() {
        // Render shader background for Now Playing view
        // Prefer multi-pass if available, fallback to single-pass
        let rect = ui.max_rect();

        if let Some(multi_shader) = &app.ui.shader_manager.multi_pass_shader {
            // Use multi-pass shader (supports BufferA-D from editor exports)
            let callback = egui_wgpu::Callback::new_paint_callback(
                rect,
                MultiPassCallback {
                    shader: multi_shader.clone(),
                    bass_energy: app.audio.bass_energy.clone(),
                    mid_energy: app.audio.mid_energy.clone(),
                    high_energy: app.audio.high_energy.clone(),
                    gamma: app.ui.shader_manager.gamma(),
                    contrast: app.ui.shader_manager.contrast(),
                    saturation: app.ui.shader_manager.saturation(),
                },
            );
            ui.painter().add(callback);
        } else if let Some(shader) = &app.ui.shader_manager.track_metadata_shader {
            // Fallback to single-pass shader (backward compatibility)
            let callback = egui_wgpu::Callback::new_paint_callback(
                rect,
                ShaderCallback {
                    shader: shader.clone(),
                    bass_energy: app.audio.bass_energy.clone(),
                    mid_energy: app.audio.mid_energy.clone(),
                    high_energy: app.audio.high_energy.clone(),
                    gamma: app.ui.shader_manager.gamma(),
                    contrast: app.ui.shader_manager.contrast(),
                    saturation: app.ui.shader_manager.saturation(),
                },
            );
            ui.painter().add(callback);
        }
        
        // No overlay - use text shadows instead for readability
        render_track_details(app, ui, current_track);
    } else {
        // Fallback: use stored current track info
        render_fallback_view(app, ui);
    }
}

/// Render error state when playback fails
fn render_error_state(ui: &mut egui::Ui, error_msg: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(100.0);
        
        ui.add(
            egui::Label::new(
                egui::RichText::new("⚠️")
                    .size(64.0)
                    .color(egui::Color32::from_rgb(255, 100, 100))
            )
        );
        
        ui.add_space(20.0);
        
        ui.add(
            egui::Label::new(
                egui::RichText::new("Playback Error")
                    .size(24.0)
                    .color(egui::Color32::from_rgb(255, 100, 100))
            )
        );
        
        ui.add_space(15.0);
        
        ui.add(
            egui::Label::new(
                egui::RichText::new(error_msg)
                    .size(16.0)
                    .color(egui::Color32::from_rgb(180, 180, 180))
            )
        );
        
        ui.add_space(20.0);
        
        ui.add(
            egui::Label::new(
                egui::RichText::new("Try selecting another track")
                    .size(14.0)
                    .color(egui::Color32::from_rgb(120, 120, 120))
            )
        );
    });
}

/// Render empty state when no track is playing
fn render_empty_state(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(150.0);
        
        ui.add(
            egui::Label::new(
                egui::RichText::new("🎵")
                    .size(64.0)
                    .color(egui::Color32::from_rgb(120, 120, 120))
            )
        );
        
        ui.add_space(20.0);
        
        ui.add(
            egui::Label::new(
                egui::RichText::new("No track playing")
                    .size(24.0)
                    .color(egui::Color32::from_rgb(150, 150, 150))
            )
        );
        
        ui.add_space(10.0);
        
        ui.add(
            egui::Label::new(
                egui::RichText::new("Search for a track or playlist to get started")
                    .size(16.0)
                    .color(egui::Color32::from_rgb(120, 120, 120))
            )
        );
    });
}

/// Render track details with large artwork and audio-reactive glow
fn render_track_details(app: &MusicPlayerApp, ui: &mut egui::Ui, track: &crate::models::Track) {
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        
        // Track title with strong outline/stroke effect
        let title_font = egui::FontId::proportional(28.0);
        let available_width = ui.available_width();
        let title_center_x = ui.cursor().min.x + available_width / 2.0;
        let title_y = ui.cursor().min.y;
        
        // Outer black stroke (thick)
        for distance in [4.0, 3.0, 2.0] {
            for angle in 0..16 {
                let rad = (angle as f32) * std::f32::consts::PI / 8.0;
                let offset_x = rad.cos() * distance;
                let offset_y = rad.sin() * distance;
                ui.painter().text(
                    egui::pos2(title_center_x + offset_x, title_y + offset_y),
                    egui::Align2::CENTER_TOP,
                    &app.audio.current_title,
                    title_font.clone(),
                    egui::Color32::BLACK,
                );
            }
        }
        // Actual white text on top
        ui.label(egui::RichText::new(&app.audio.current_title).size(28.0).strong().color(egui::Color32::WHITE));
        ui.add_space(10.0);
        
        // Artist name with strong outline
        let artist_font = egui::FontId::proportional(20.0);
        let artist_center_x = ui.cursor().min.x + available_width / 2.0;
        let artist_y = ui.cursor().min.y;
        
        // Outer black stroke
        for distance in [3.0, 2.0] {
            for angle in 0..16 {
                let rad = (angle as f32) * std::f32::consts::PI / 8.0;
                let offset_x = rad.cos() * distance;
                let offset_y = rad.sin() * distance;
                ui.painter().text(
                    egui::pos2(artist_center_x + offset_x, artist_y + offset_y),
                    egui::Align2::CENTER_TOP,
                    &track.user.username,
                    artist_font.clone(),
                    egui::Color32::BLACK,
                );
            }
        }
        ui.label(egui::RichText::new(&track.user.username).size(20.0).color(egui::Color32::from_rgb(255, 85, 0)));
        
        ui.add_space(100.0);
        
        let artwork_size = 400.0;
        
        // Use real artwork if loaded, otherwise placeholder
        let texture_to_use = if app.ui.artwork_texture.is_some() && app.audio.current_track_id == Some(track.id) {
            &app.ui.artwork_texture
        } else {
            &app.ui.no_artwork_texture
        };
        
        if let Some(texture) = texture_to_use {
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(artwork_size, artwork_size), egui::Sense::hover());
            
            // Draw audio-reactive glow if real artwork
            if app.ui.artwork_texture.is_some() && app.audio.current_track_id == Some(track.id) {
                render_artwork_glow(ui, rect, app);
            }
            
            // Draw artwork
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            // Fallback: Gray box
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(artwork_size, artwork_size), egui::Sense::hover());
            ui.painter().rect_filled(
                rect,
                20.0,
                egui::Color32::from_rgb(60, 60, 65),
            );
        }
    });
}

/// Render fallback view using stored track info
fn render_fallback_view(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    // Render shader background for Now Playing view
    // Prefer multi-pass if available, fallback to single-pass
    let rect = ui.max_rect();

    if let Some(multi_shader) = &app.ui.shader_manager.multi_pass_shader {
        // Use multi-pass shader (supports BufferA-D from editor exports)
        let callback = egui_wgpu::Callback::new_paint_callback(
            rect,
            MultiPassCallback {
                shader: multi_shader.clone(),
                bass_energy: app.audio.bass_energy.clone(),
                mid_energy: app.audio.mid_energy.clone(),
                high_energy: app.audio.high_energy.clone(),
                gamma: app.ui.shader_manager.gamma(),
                contrast: app.ui.shader_manager.contrast(),
                saturation: app.ui.shader_manager.saturation(),
            },
        );
        ui.painter().add(callback);
    } else if let Some(shader) = &app.ui.shader_manager.track_metadata_shader {
        // Fallback to single-pass shader (backward compatibility)
        let callback = egui_wgpu::Callback::new_paint_callback(
            rect,
            ShaderCallback {
                shader: shader.clone(),
                bass_energy: app.audio.bass_energy.clone(),
                mid_energy: app.audio.mid_energy.clone(),
                high_energy: app.audio.high_energy.clone(),
                gamma: app.ui.shader_manager.gamma(),
                contrast: app.ui.shader_manager.contrast(),
                saturation: app.ui.shader_manager.saturation(),
            },
        );
        ui.painter().add(callback);
    }
    
    // No overlay - use text outlines for readability
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        
        // Track title with strong outline
        let title_font = egui::FontId::proportional(28.0);
        let available_width = ui.available_width();
        let title_center_x = ui.cursor().min.x + available_width / 2.0;
        let title_y = ui.cursor().min.y;
        
        for distance in [4.0, 3.0, 2.0] {
            for angle in 0..16 {
                let rad = (angle as f32) * std::f32::consts::PI / 8.0;
                let offset_x = rad.cos() * distance;
                let offset_y = rad.sin() * distance;
                ui.painter().text(
                    egui::pos2(title_center_x + offset_x, title_y + offset_y),
                    egui::Align2::CENTER_TOP,
                    &app.audio.current_title,
                    title_font.clone(),
                    egui::Color32::BLACK,
                );
            }
        }
        ui.label(egui::RichText::new(&app.audio.current_title).size(28.0).strong().color(egui::Color32::WHITE));
        ui.add_space(10.0);
        
        // Artist name with outline
        let artist_font = egui::FontId::proportional(20.0);
        let artist_center_x = ui.cursor().min.x + available_width / 2.0;
        let artist_y = ui.cursor().min.y;
        
        for distance in [3.0, 2.0] {
            for angle in 0..16 {
                let rad = (angle as f32) * std::f32::consts::PI / 8.0;
                let offset_x = rad.cos() * distance;
                let offset_y = rad.sin() * distance;
                ui.painter().text(
                    egui::pos2(artist_center_x + offset_x, artist_y + offset_y),
                    egui::Align2::CENTER_TOP,
                    &app.audio.current_artist,
                    artist_font.clone(),
                    egui::Color32::BLACK,
                );
            }
        }
        ui.label(egui::RichText::new(&app.audio.current_artist).size(20.0).color(egui::Color32::from_rgb(255, 85, 0)));
        
        ui.add_space(50.0);
        
        let artwork_size = 400.0;
        
        let texture_to_use = if app.ui.artwork_texture.is_some() {
            &app.ui.artwork_texture
        } else {
            &app.ui.no_artwork_texture
        };
        
        if let Some(texture) = texture_to_use {
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(artwork_size, artwork_size), egui::Sense::hover());
            
            // Draw audio-reactive glow
            if app.ui.artwork_texture.is_some() {
                render_artwork_glow(ui, rect, app);
            }
            
            // Draw artwork
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(artwork_size, artwork_size), egui::Sense::hover());
            ui.painter().rect_filled(
                rect,
                20.0,
                egui::Color32::from_rgb(60, 60, 65),
            );
        }
    });
}

/// Render INTENSE audio-reactive glow around artwork (FIRE & THUNDER EDITION)
fn render_artwork_glow(ui: &mut egui::Ui, rect: egui::Rect, app: &MusicPlayerApp) {
    let [r, g, b, _] = app.ui.artwork_dominant_color.to_array();
    
    // Subtle audio reactive boost near edges (1.0-1.4x)
    let audio_boost = 1.0 + (app.ui.audio_amplitude * 0.4);
    
    // 4 glow layers for subtle edge glow (reduced from 8)
    for i in 0..6 {
        let layer_idx = i as f32;
        let expansion = (layer_idx + 1.0) * 2.1 * audio_boost;  // 5.0 -> 2.0 (subtle near edges)
        let base_alpha = (150.0 - (layer_idx * 25.0)).max(0.0) as u8;  // Softer falloff
        let alpha = ((base_alpha as f32) * audio_boost.min(1.5)) as u8;
        
        let glow_rect = rect.expand(expansion);
        ui.painter().rect_filled(
            glow_rect,
            12.0,
            egui::Color32::from_rgba_premultiplied(r, g, b, alpha),
        );
    }
}


