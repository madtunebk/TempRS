use eframe::egui;
use egui::Layout;

use crate::app::player_app::MusicPlayerApp;
use crate::app_state::RepeatMode;
use crate::utils::formatting::format_duration;


/// Render the music player controls in the footer panel
/// NOTE: Player bar layout - 3 columns (controls, progress, social+volume)
/// Called from layout.rs footer panel. Uses centered_and_justified for vertical/horizontal centering
pub fn render_player(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    use crate::app::player_app::MainTab;

    // Make entire footer clickable
    let footer_rect = ui.max_rect();
    let footer_resp = ui.interact(
        footer_rect,
        ui.id().with("player_footer_click"),
        egui::Sense::click(),
    );

    if footer_resp.clicked() {
        app.selected_tab = MainTab::NowPlaying;
    }
    if footer_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    // Add horizontal padding and center all controls
    ui.horizontal(|ui| {
        // Fixed window width is 1480px, controls take ~1080px
        // Calculate padding to perfectly center: (1480 - 1080) / 2 = 200px
        let total_width = ui.available_width();
        let controls_width = 1080.0;  // Approximate width of all controls
        let pad = (total_width - controls_width).max(0.0) / 2.0;
        
        ui.add_space(pad);  // Left padding
        
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 20.0;
            render_compact_social_buttons(app, ui);
            render_all_controls(app, ui);
            render_progress_bar(app, ui);
            render_volume_controls(app, ui);
        });
        
        ui.add_space(pad);  // Right padding
    });
}


// NOTE: Player bar controls - simple horizontal layout without extra nesting
fn render_all_controls(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.x = 10.0;
    
    // Check queue size and navigation availability
    let queue_size = app.playback_queue.current_queue.len();
        let is_single_track = queue_size <= 1;
        
        let can_prev = app.playback_queue.current_index
            .map(|idx| idx > 0)
            .unwrap_or(false);
        
        let can_next = app.playback_queue.current_index
            .map(|idx| idx < queue_size - 1)
            .unwrap_or(false);

        // Shuffle button - disabled for single track
        let is_repeat_one = app.repeat_mode == RepeatMode::One;
        let shuffle_enabled = !is_single_track && !is_repeat_one;
        let shuffle_color = if app.shuffle_mode && shuffle_enabled {
            egui::Color32::from_rgb(255, 138, 43)
        } else if is_single_track {
            egui::Color32::from_rgb(100, 100, 100)
        } else {
            egui::Color32::from_rgb(180, 180, 180)
        };
        
        let shuffle_btn = ui.add_enabled(
            shuffle_enabled,
            egui::Button::new(egui::RichText::new("🔀").size(12.0).color(shuffle_color))
                .fill(egui::Color32::TRANSPARENT)
                .corner_radius(50.0)
                .min_size(egui::vec2(40.0, 40.0))
        );
        if shuffle_btn.clicked() {
            app.toggle_shuffle();
        }

        // Previous button - disabled for single track or at beginning
        let prev_enabled = !is_single_track && can_prev;
        let prev_fill = if prev_enabled {
            egui::Color32::from_rgb(45, 45, 50)
        } else {
            egui::Color32::from_rgb(35, 35, 40)
        };
        
        let prev_btn = ui.add_enabled(
            prev_enabled,
            egui::Button::new(egui::RichText::new("⏮").size(14.0).color(egui::Color32::WHITE))
                .fill(prev_fill)
                .corner_radius(50.0)
                .min_size(egui::vec2(40.0, 40.0))
        );
        if prev_btn.clicked() {
            app.play_previous();
        }
        
        // Stop button
        let has_track = app.current_track_id.is_some();
        let stop_fill = if has_track {
            egui::Color32::from_rgb(45, 45, 50)
        } else {
            egui::Color32::from_rgb(35, 35, 40)
        };
        
        let stop_btn = ui.add_enabled(
            has_track,
            egui::Button::new(egui::RichText::new("⏹").size(14.0).color(egui::Color32::WHITE))
                .fill(stop_fill)
                .corner_radius(50.0)
                .min_size(egui::vec2(40.0, 40.0))
        );
        if stop_btn.clicked() {
            log::info!("[UI] Stop button clicked - stopping playback");
            app.stop_playback();
        }
        
        // Play/Pause button
        let has_track = app.current_track_id.is_some();
        let play_icon = if app.is_playing { "⏸" } else { "▶" };
        let play_fill = if app.is_playing {
            egui::Color32::from_rgb(255, 85, 0)
        } else if has_track {
            egui::Color32::from_rgb(45, 45, 50)
        } else {
            egui::Color32::from_rgb(35, 35, 40)
        };
        
        let play_btn = ui.add_enabled(
            has_track,
            egui::Button::new(egui::RichText::new(play_icon).size(14.0).color(egui::Color32::WHITE))
                .fill(play_fill)
                .corner_radius(50.0)
                .min_size(egui::vec2(40.0, 40.0))
        );
        if play_btn.clicked() {
            log::info!("[UI] Play/Pause clicked - current is_playing: {}", app.is_playing);
            app.toggle_playback();
        }
        
        // Next button - disabled for single track or at end
        let next_enabled = !is_single_track && can_next;
        let next_fill = if next_enabled {
            egui::Color32::from_rgb(45, 45, 50)
        } else {
            egui::Color32::from_rgb(35, 35, 40)
        };
        
        let next_btn = ui.add_enabled(
            next_enabled,
            egui::Button::new(egui::RichText::new("⏭").size(14.0).color(egui::Color32::WHITE))
                .fill(next_fill)
                .corner_radius(50.0)
                .min_size(egui::vec2(40.0, 40.0))
        );
        if next_btn.clicked() {
            app.play_next();
        }
        
        // Repeat button - disabled for single track
        let repeat_enabled = !is_single_track;
        let (repeat_icon, repeat_color) = match app.repeat_mode {
            RepeatMode::None => ("🔁", if is_single_track {
                egui::Color32::from_rgb(100, 100, 100)
            } else {
                egui::Color32::from_rgb(180, 180, 180)
            }),
            RepeatMode::One => ("🔂", egui::Color32::from_rgb(255, 138, 43)),
            RepeatMode::All => ("🔁", egui::Color32::from_rgb(255, 138, 43)),
        };
        
        let repeat_btn = ui.add_enabled(
            repeat_enabled,
            egui::Button::new(egui::RichText::new(repeat_icon).size(12.0).color(repeat_color))
                .fill(egui::Color32::TRANSPARENT)
                .corner_radius(50.0)
                .min_size(egui::vec2(40.0, 40.0))
        );
        if repeat_btn.clicked() {
            app.cycle_repeat_mode();
        }
}

fn render_volume_controls(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.x = 10.0;
    
    // Speaker button - toggles popup
    let mute_icon = if app.muted { "🔇" } else { "🔊" };
    let speaker_color = if app.show_volume_popup {
        egui::Color32::from_rgb(255, 120, 40)  // Orange when popup is open
    } else {
        egui::Color32::from_rgb(160, 160, 160)
    };
    
    let volume_btn = ui.add(
        egui::Button::new(egui::RichText::new(mute_icon).size(14.0).color(speaker_color))
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::NONE)
            .corner_radius(50.0)
            .min_size(egui::vec2(32.0, 32.0))
    );
    
    // Right-click to mute/unmute, left-click to toggle popup
    if volume_btn.clicked_by(egui::PointerButton::Secondary) {
        app.toggle_mute();
    } else if volume_btn.clicked() {
        app.show_volume_popup = !app.show_volume_popup;
    }
    
    // Show vertical popup slider above speaker icon
    if app.show_volume_popup {
        let popup_id = ui.id().with("volume_popup");
        let button_rect = volume_btn.rect;
        
        // Position popup above the button
        let popup_width = 50.0;
        let popup_height = 140.0;
        let popup_pos = egui::pos2(
            button_rect.center().x - popup_width / 2.0,
            button_rect.min.y - popup_height - 10.0,  // 10px gap above button
        );
        
        let popup_rect = egui::Rect::from_min_size(popup_pos, egui::vec2(popup_width, popup_height));
        
        // Check if clicking outside popup to close it
        let popup_response = ui.interact(
            popup_rect.expand(5.0),  // Slightly larger hit area
            popup_id,
            egui::Sense::click(),
        );
        
        if ui.input(|i| i.pointer.any_click()) && !popup_response.hovered() && !volume_btn.hovered() {
            app.show_volume_popup = false;
        }
        
        // Draw popup background with shadow
        let painter = ui.painter();
        
        // Shadow layers
        for i in 0..3 {
            let offset = (3 - i) as f32 * 2.0;
            let alpha = 20 + i * 10;
            painter.rect_filled(
                popup_rect.translate(egui::vec2(0.0, offset)),
                8.0,
                egui::Color32::from_rgba_premultiplied(0, 0, 0, alpha),
            );
        }
        
        // Main popup background
        painter.rect_filled(
            popup_rect,
            8.0,
            egui::Color32::from_rgb(35, 35, 40),
        );
        painter.rect_stroke(
            popup_rect,
            8.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 65)),
            egui::epaint::StrokeKind::Outside,
        );
        
        // Vertical slider area
        let slider_padding = 15.0;
        let slider_width = 6.0;
        let slider_height = popup_height - slider_padding * 2.0 - 25.0;  // Leave room for percentage text
        
        let slider_x = popup_rect.center().x - slider_width / 2.0;
        let slider_top = popup_rect.min.y + slider_padding;
        let slider_bottom = slider_top + slider_height;
        
        let slider_rect = egui::Rect::from_min_max(
            egui::pos2(slider_x, slider_top),
            egui::pos2(slider_x + slider_width, slider_bottom),
        );
        
        // Handle interaction
        let mut volume = app.volume;
        let slider_response = ui.interact(
            slider_rect.expand2(egui::vec2(15.0, 0.0)),  // Wider hit area
            popup_id.with("slider"),
            egui::Sense::click_and_drag(),
        );
        
        if slider_response.dragged() || slider_response.clicked() {
            if let Some(pos) = slider_response.interact_pointer_pos() {
                // Calculate volume (inverted: top = 1.0, bottom = 0.0)
                let raw_volume = 1.0 - ((pos.y - slider_top) / slider_height).clamp(0.0, 1.0);
                
                // Snap to 5% increments
                volume = (raw_volume * 20.0).round() / 20.0;
                app.set_volume(volume);
                
                // Unmute if adjusting volume while muted
                if app.muted {
                    app.muted = false;
                }
            }
        }
        
        // Background track
        painter.rect_filled(slider_rect, 3.0, egui::Color32::from_rgb(50, 50, 55));
        
        // Active volume bar (fills from bottom to volume level)
        let volume_height = slider_height * volume;
        let volume_bar = egui::Rect::from_min_max(
            egui::pos2(slider_x, slider_bottom - volume_height),
            egui::pos2(slider_x + slider_width, slider_bottom),
        );
        painter.rect_filled(volume_bar, 3.0, egui::Color32::from_rgb(255, 120, 40));
        
        // Handle at volume position
        let handle_y = slider_bottom - volume_height;
        let handle_center = egui::pos2(popup_rect.center().x, handle_y);
        let handle_radius = 8.0;
        
        // Glow effect
        painter.circle_filled(
            handle_center,
            handle_radius + 3.0,
            egui::Color32::from_rgba_premultiplied(255, 120, 40, 60),
        );
        painter.circle_filled(
            handle_center,
            handle_radius,
            egui::Color32::WHITE,
        );
        
        // Volume percentage text at bottom
        let percent_text = format!("{:.0}%", volume * 100.0);
        painter.text(
            egui::pos2(popup_rect.center().x, popup_rect.max.y - 12.0),
            egui::Align2::CENTER_CENTER,
            percent_text,
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgb(200, 200, 200),
        );
    }
}

fn render_progress_bar(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    let position = app.get_position();
    let duration = app.get_duration();
    let position_secs = position.as_secs_f32();
    let duration_secs = duration.as_secs_f32();
    
    // Check if track is loaded and ready for seeking
    let can_seek = app.current_track_id.is_some() && app.track_start_time.is_some();
    
    ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        
        // Current time
        ui.label(
            egui::RichText::new(format_duration(position_secs))
            .size(11.0)
            .color(egui::Color32::from_rgb(170, 170, 170))
        );
        
        // Progress bar - wider, sleeker
        let width = 420.0_f32;
        let handle_radius = 6.0;
        
        // Disable interaction if can't seek
        let sense = if can_seek {
            egui::Sense::click_and_drag()
        } else {
            egui::Sense::hover()
        };
        
        let (response, painter) = ui.allocate_painter(
            egui::vec2(width.max(200.0), 20.0),
            sense,
        );
        
        let rect = response.rect;
        let bar_height = 4.0;
        let vertical_center = rect.center().y;
        
        let bar_left = rect.min.x + handle_radius;
        let bar_right = rect.max.x - handle_radius;
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(bar_left, vertical_center - bar_height / 2.0),
            egui::pos2(bar_right, vertical_center + bar_height / 2.0),
        );
        
        // Background track
        painter.rect_filled(bar_rect, 2.0, egui::Color32::from_rgb(70, 70, 75));
        
        let actual_position = app.audio_controller.get_position().as_secs_f32();
        let actual_progress = if duration_secs > 0.0 {
            actual_position / duration_secs
        } else {
            0.0
        };
        
        let (display_progress, is_dragging) = if can_seek && response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let drag_progress = ((pos.x - bar_left) / (bar_right - bar_left)).clamp(0.0, 1.0);
                app.seek_target_pos = Some(std::time::Duration::from_secs_f32(duration_secs * drag_progress));
                app.is_seeking = true;
                (drag_progress, true)
            } else {
                (actual_progress, false)
            }
        } else if app.is_seeking {
            if let Some(seek_target) = app.seek_target_pos {
                let seek_progress = seek_target.as_secs_f32() / duration_secs.max(0.001);
                let diff = (seek_progress - actual_progress).abs();
                if diff < 0.1 / duration_secs.max(1.0) {
                    app.seek_target_pos = None;
                    app.is_seeking = false;
                    (actual_progress, false)
                } else {
                    (seek_progress, false)
                }
            } else {
                app.is_seeking = false;
                (actual_progress, false)
            }
        } else {
            (actual_progress, false)
        };
        
        let progress_color = if is_dragging || app.is_seeking {
            egui::Color32::from_rgb(80, 150, 255)   // Blue when seeking
        } else {
            egui::Color32::from_rgb(255, 100, 30)   // Orange normal
        };
        
        // Show current playback position (dimmed if seeking)
        if is_dragging || app.is_seeking {
            let current_rect = egui::Rect::from_min_max(
                bar_rect.min,
                egui::pos2(bar_left + (bar_right - bar_left) * actual_progress, bar_rect.max.y),
            );
            painter.rect_filled(current_rect, 2.0, egui::Color32::from_rgb(100, 100, 105));
        }
        
        // Active progress
        let progress_rect = egui::Rect::from_min_max(
            bar_rect.min,
            egui::pos2(bar_left + (bar_right - bar_left) * display_progress, bar_rect.max.y),
        );
        painter.rect_filled(progress_rect, 2.0, progress_color);
        
        // Handle - only on hover or drag
        if response.hovered() || is_dragging || app.is_seeking {
            let handle_x = bar_left + (bar_right - bar_left) * display_progress;
            let handle_center = egui::pos2(handle_x, vertical_center);
            let handle_radius_size = if is_dragging { 7.0 } else { 6.0 };
            painter.circle_filled(handle_center, handle_radius_size, progress_color);
            painter.circle_stroke(
                handle_center,
                handle_radius_size,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 255, 255)),
            );
        }
        
        // Preview time on drag
        if is_dragging || app.is_seeking {
            let preview_secs = duration_secs * display_progress;
            let preview_text = format_duration(preview_secs);
            let handle_x = bar_left + (bar_right - bar_left) * display_progress;
            let text_pos = egui::pos2(handle_x, vertical_center - 16.0);
            painter.text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                preview_text,
                egui::FontId::proportional(11.0),
                progress_color,
            );
        }
        
        if response.drag_stopped() {
            if let Some(seek_target) = app.seek_target_pos {
                app.seek_to(seek_target);
            }
        }
        
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let seek_progress = ((pos.x - bar_left) / (bar_right - bar_left)).clamp(0.0, 1.0);
                let seek_position = std::time::Duration::from_secs_f32(duration_secs * seek_progress);
                app.seek_to(seek_position);
            }
        }
        
        // Duration time
        ui.label(
            egui::RichText::new(format_duration(duration_secs))
            .size(11.0)
            .color(egui::Color32::from_rgb(170, 170, 170))
        );
    });
}

/// Compact social buttons for player bar (icon-only to save space)
fn render_compact_social_buttons(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    let has_track = app.current_track_id.is_some();
    
    // Like button (icon only, circular)
    let (like_icon, like_color) = if app.is_current_track_liked() {
        ("❤", egui::Color32::from_rgb(255, 85, 0))  // Orange filled heart when liked
    } else {
        ("♡", egui::Color32::from_rgb(160, 160, 160))  // Gray outline heart when not liked
    };
    
    let like_btn = ui.add_enabled(
        has_track,
        egui::Button::new(
            egui::RichText::new(like_icon)
                .size(16.0)
                .color(like_color)
        )
        .fill(egui::Color32::from_rgb(40, 40, 45))
        .corner_radius(50.0)
        .min_size(egui::vec2(32.0, 32.0))
    );
    
    if like_btn.clicked() {
        app.toggle_current_track_like();
    }
    
    if like_btn.hovered() {
        like_btn.on_hover_text(if app.is_current_track_liked() { "Unlike" } else { "Like" });
    }
    
    // Share button (icon only, circular)
    let share_btn = ui.add_enabled(
        has_track,
        egui::Button::new(
            egui::RichText::new("⤴")
                .size(16.0)
                .color(egui::Color32::from_rgb(160, 160, 160))
        )
        .fill(egui::Color32::from_rgb(40, 40, 45))
        .corner_radius(50.0)
        .min_size(egui::vec2(32.0, 32.0))
    );
    
    if share_btn.clicked() {
        app.share_current_track();
    }
    
    if share_btn.hovered() {
        share_btn.on_hover_text("Share - Copy URL");
    }
}


