// This module has been refactored:
// - Formatting utilities moved to utils/formatting.rs
// - Layout and responsive sizing moved to app/layout.rs
// - Remaining unused UI helpers removed for cleaner codebase

use eframe::egui;
use egui::Color32;
use crate::ui_components::colors::*;

/// Render like button only in top-left corner of artwork
/// Share/copy functionality is available in the bottom player bar
/// Returns (like_clicked, share_clicked) - share_clicked always false now
pub fn render_social_buttons(
    app: &mut crate::app::player_app::MusicPlayerApp,
    ui: &mut egui::Ui,
    artwork_rect: egui::Rect,
    track_id: u64,
    _permalink_url: Option<&String>,  // Not used anymore, kept for API compatibility
) -> (bool, bool) {
    use egui::{Vec2, Sense};
    
    let button_size = 32.0;
    let top_left_padding = 4.0;
    
    // Like button in top-left corner (only button on artwork now)
    // Use artwork_rect position as unique context to avoid ID clashes when same track appears multiple times
    let like_pos = artwork_rect.min + Vec2::new(top_left_padding, top_left_padding);
    let like_rect = egui::Rect::from_min_size(like_pos, Vec2::new(button_size, button_size));
    let unique_id = ui.id().with(("like", track_id, artwork_rect.min.x as i32, artwork_rect.min.y as i32));
    let like_response = ui.interact(like_rect, unique_id, Sense::click());
    
    let is_liked = app.content.liked_track_ids.contains(&track_id);
    let (like_icon, like_bg_color) = if is_liked {
        ("❤", Color32::from_rgba_premultiplied(255, 85, 0, 200))  // Orange filled heart
    } else if like_response.hovered() {
        ("♡", Color32::from_rgba_premultiplied(255, 255, 255, 200))  // White outline on hover
    } else {
        ("♡", Color32::from_rgba_premultiplied(0, 0, 0, 150))  // Dark outline normally
    };
    
    ui.painter().circle_filled(like_rect.center(), button_size / 2.0, like_bg_color);
    ui.painter().text(
        like_rect.center(),
        egui::Align2::CENTER_CENTER,
        like_icon,
        egui::FontId::proportional(18.0),
        Color32::WHITE,
    );
    
    let like_clicked = like_response.clicked();
    if like_clicked {
        app.toggle_like(track_id);
    }
    
    if like_response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    
    // Share button removed - accessible only from bottom player bar
    (like_clicked, false)
}

/// Truncate text to max length with ellipsis
pub fn truncate_text(text: &str, max_len: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_len {
        text.to_string()
    } else {
        chars.iter().take(max_len).collect::<String>() + "..."
    }
}

/// Render section header with title and optional action button (used in home/library)
pub fn render_section_header(ui: &mut egui::Ui, title: &str, action_text: Option<&str>) -> bool {
    let mut clicked = false;
    
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        
        // Section title
        ui.label(
            egui::RichText::new(title)
                .size(24.0)
                .color(egui::Color32::WHITE)
                .strong()
        );
        
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(20.0);
            
            if let Some(text) = action_text {
                let btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new(text)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(180, 180, 180))
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(egui::Stroke::NONE)
                    .corner_radius(4.0)
                );
                
                if btn.clicked() {
                    clicked = true;
                }
            }
        });
    });
    
    clicked
}

/// Render track card with artwork and metadata (returns click states)
/// Used by home and search screens for consistent grid display
/// Standardized: 1:1 aspect ratio, 8px padding, smooth hover transitions
/// Returns: (clicked, shift_clicked, right_clicked)
pub fn render_track_card(
    app: &mut crate::app::player_app::MusicPlayerApp,
    ui: &mut egui::Ui,
    track: &crate::app::playlists::Track,
    card_size: f32,
) -> (bool, bool, bool) {
    use eframe::egui::{Vec2, Sense, Color32};
    
    // Card standardization: fixed padding and metadata height
    let card_padding = 8.0;
    let metadata_height = 60.0; // Increased from 50 to prevent overlap
    let full_height = card_size + metadata_height;
    
    let (rect, response) = ui.allocate_exact_size(Vec2::new(card_size, full_height), Sense::click());
    
    let clicked = response.clicked();
    let shift_clicked = response.clicked() && ui.input(|i| i.modifiers.shift);
    let right_clicked = response.secondary_clicked();
    
    // Enhanced hover effect with subtle background and border
    if response.hovered() {
        // Subtle background fill
        ui.painter().rect_filled(
            rect,
            8.0,
            Color32::from_rgb(26, 26, 26),
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
        let triangle_offset_x = 3.0; // Slight offset to visually center the triangle
        ui.painter().text(
            center + egui::Vec2::new(triangle_offset_x, 0.0),
            egui::Align2::CENTER_CENTER,
            "▶",
            egui::FontId::proportional(triangle_size),
            Color32::WHITE,
        );
    }
    
    // Social buttons in top-left corner (like/share) - using reusable function
    let (_like_clicked, _share_clicked) = render_social_buttons(
        app,
        ui,
        artwork_rect,
        track.id,
        track.permalink_url.as_ref(),
    );
    
    // Metadata section with consistent padding
    let metadata_y = rect.min.y + card_size + card_padding;
    
    // Track title (truncated to fit card width)
    let title_pos = egui::pos2(rect.min.x + card_padding, metadata_y);
    let title_text = truncate_text(&track.title, 26); // Increased for larger cards
    ui.painter().text(
        title_pos,
        egui::Align2::LEFT_TOP,
        title_text,
        egui::FontId::proportional(12.0), // Reduced for tighter spacing
        Color32::from_rgb(240, 240, 240),
    );
    
    // Artist name (truncated to fit, with increased spacing)
    let artist_pos = egui::pos2(rect.min.x + card_padding, metadata_y + 22.0);
    let artist_text = truncate_text(&track.user.username, 28); // Increased for larger cards
    ui.painter().text(
        artist_pos,
        egui::Align2::LEFT_TOP,
        artist_text,
        egui::FontId::proportional(11.0), // Increased from 9
        Color32::from_rgb(160, 160, 160),
    );
    
    (clicked && !shift_clicked, shift_clicked, right_clicked)
}

/// Render grid layout with auto-fit columns (shared pattern for search/home)
/// Uses full available width with fixed left padding only
pub fn calculate_grid_layout(available_width: f32, item_size: f32, spacing: f32) -> (usize, f32) {
    let left_padding = 20.0;
    let usable_width = available_width - (left_padding * 2.0);
    let items_per_row = ((usable_width + spacing) / (item_size + spacing)).floor().max(1.0) as usize;
    
    (items_per_row, left_padding)
}

/// Render grey placeholder card for empty grid slots
/// Maintains consistent height with actual track cards
pub fn render_placeholder_card(ui: &mut egui::Ui, card_size: f32) {
    use egui::{Color32, Vec2};
    
    let metadata_height = 60.0;
    let full_height = card_size + metadata_height;
    let rect = ui.allocate_exact_size(Vec2::new(card_size, full_height), egui::Sense::hover()).0;
    
    // Grey placeholder box (matches artwork size only)
    let artwork_rect = egui::Rect::from_min_size(
        rect.min,
        Vec2::new(card_size, card_size),
    );
    
    ui.painter().rect_filled(
        artwork_rect,
        6.0,
        Color32::from_rgb(50, 50, 55),
    );
}

/// Render empty state card with icon, title, and message
/// Maintains consistent height with track card rows
pub fn render_empty_state(ui: &mut egui::Ui, icon: &str, title: &str, message: &str) {
    use egui::{Color32, Vec2};
    
    ui.vertical_centered(|ui| {
        // Match track card row height: 220px card + 60px metadata = 280px
        let card_height = 280.0;
        let card_width = ui.available_width() * 0.6;
        
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(card_width, card_height),
            egui::Sense::hover(),
        );
        
        // Draw card background
        ui.painter().rect_filled(
            rect,
            12.0,
            Color32::from_rgb(45, 45, 50),
        );
        ui.painter().rect_stroke(
            rect,
            12.0,
            egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 65)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // Draw content centered in the rect
        let center = rect.center();
        let icon_pos = center - egui::Vec2::new(0.0, 50.0);
        let title_pos = center;
        let msg_pos = center + egui::Vec2::new(0.0, 30.0);
        
        // Icon
        ui.painter().text(
            icon_pos,
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(48.0),
            Color32::WHITE,
        );
        
        // Title
        ui.painter().text(
            title_pos,
            egui::Align2::CENTER_CENTER,
            title,
            egui::FontId::proportional(16.0),
            Color32::from_rgb(200, 200, 200),
        );
        
        // Message
        ui.painter().text(
            msg_pos,
            egui::Align2::CENTER_CENTER,
            message,
            egui::FontId::proportional(13.0),
            Color32::GRAY,
        );
    });
}

/// Render centered pagination controls with prev/next buttons and page indicator
/// Returns true if page changed
pub fn render_pagination_controls(
    ui: &mut egui::Ui,
    current_page: &mut usize,
    total_items: usize,
    page_size: usize,
) -> bool {
    let total_pages = (total_items + page_size - 1) / page_size;
    
    if total_pages <= 1 {
        return false;
    }
    
    let mut page_changed = false;
    
    // Add horizontal padding and center pagination controls
    ui.horizontal(|ui| {
        // Fixed controls width: prev(40) + space(15) + text(~100) + space(15) + next(40) ≈ 210px
        let total_width = ui.available_width();
        let controls_width = 210.0;
        let pad = (total_width - controls_width).max(0.0) / 2.0;
        
        ui.add_space(pad);
        
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            // Previous button (circular)
            let has_prev = *current_page > 0;
            let prev_btn = ui.add_enabled(
                has_prev,
                egui::Button::new(
                    egui::RichText::new("←")
                        .size(18.0)
                        .color(if has_prev { Color32::WHITE } else { Color32::DARK_GRAY })
                )
                .fill(if has_prev { Color32::from_rgb(45, 45, 50) } else { Color32::from_rgb(25, 25, 30) })
                .min_size(egui::vec2(40.0, 40.0))
                .corner_radius(20.0)
            );
            
            if prev_btn.clicked() && has_prev {
                *current_page -= 1;
                page_changed = true;
            }
            
            ui.add_space(15.0);
            
            // Page indicator
            ui.label(
                egui::RichText::new(format!("Page {} of {}", *current_page + 1, total_pages))
                    .size(14.0)
                    .color(Color32::from_rgb(180, 180, 180))
            );
            
            ui.add_space(15.0);
            
            // Next button (circular)
            let has_next = *current_page < total_pages - 1;
            let next_btn = ui.add_enabled(
                has_next,
                egui::Button::new(
                    egui::RichText::new("→")
                        .size(18.0)
                        .color(if has_next { Color32::WHITE } else { Color32::DARK_GRAY })
                )
                .fill(if has_next { Color32::from_rgb(45, 45, 50) } else { Color32::from_rgb(25, 25, 30) })
                .min_size(egui::vec2(40.0, 40.0))
                .corner_radius(20.0)
            );
            
            if next_btn.clicked() && has_next {
                *current_page += 1;
                page_changed = true;
            }
        });
        
        ui.add_space(pad);
    });
    
    page_changed
}
