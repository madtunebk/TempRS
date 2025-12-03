use eframe::egui;
use crate::app::player_app::{MusicPlayerApp, MainTab};
use crate::ui_components::{header::render_header, player::render_player};

/// Main entry point for rendering the app UI - routes to appropriate screen
/// NOTE: Parent layout wrapper for all screens - called from player_app.rs
/// To change global layout behavior, modify render_layout_with_content() below
pub fn render_with_layout(app: &mut MusicPlayerApp, ctx: &egui::Context) {
    // Keyboard shortcuts for navigation (Ctrl + Key)
    ctx.input(|i| {
        if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
            app.selected_tab = MainTab::Home;
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::N) && app.current_track_id.is_some() {
            app.selected_tab = MainTab::NowPlaying;
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::H) {
            app.selected_tab = MainTab::History;
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
            app.selected_tab = MainTab::Suggestions;
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::L) {
            app.selected_tab = MainTab::Likes;
            // Refresh likes data on keyboard shortcut
            app.fetch_likes();
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::P) {
            app.selected_tab = MainTab::Playlists;
            // Refresh playlists data on keyboard shortcut
            app.fetch_playlists();
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::R) && (!app.search_results_tracks.is_empty() || !app.search_results_playlists.is_empty()) {
            app.selected_tab = MainTab::Search;
        }
    });
    
    // Load no_artwork texture if not already loaded
    if app.no_artwork_texture.is_none() {
        load_no_artwork_texture(app, ctx);
    }
    
    // Show sidebar only when multiple tracks (playlist) are loaded AND on Now Playing tab
    let show_sidebar = app.playback_queue.current_queue.len() > 1 
        && app.selected_tab == MainTab::NowPlaying;
    
    match app.selected_tab {
        MainTab::Home => {
            render_layout_with_content(app, ctx, false, |app, ui, _ctx| {
                crate::screens::render_home_view(app, ui);
            });
        }
        MainTab::NowPlaying => {
            render_layout_with_content(app, ctx, show_sidebar, |app, ui, _ctx| {
                crate::screens::render_now_playing_view(app, ui, ctx);
            });
        }
        MainTab::Search => {
            render_layout_with_content(app, ctx, false, |app, ui, ctx| {
                crate::screens::render_search_view(app, ui, ctx);
            });
        }
        MainTab::History => {
            render_layout_with_content(app, ctx, false, |app, ui, _ctx| {
                crate::screens::render_history_view(app, ui);
            });
        }
        MainTab::Suggestions => {
            render_layout_with_content(app, ctx, false, |app, ui, _ctx| {
                crate::screens::render_suggestions_view(app, ui);
            });
        }
        MainTab::Likes => {
            render_layout_with_content(app, ctx, false, |app, ui, ctx| {
                crate::screens::render_likes_view(app, ui, ctx);
            });
        }
        MainTab::Playlists => {
            render_layout_with_content(app, ctx, false, |app, ui, ctx| {
                crate::screens::render_user_playlists_view(app, ui, ctx);
            });
        }
    }
}

/// Internal helper - renders header, footer, sidebar, and central content
/// NOTE: This is where the parent layout structure is applied to all screens
/// Modify the CentralPanel section below to change content area layout/centering
fn render_layout_with_content<F>(
    app: &mut MusicPlayerApp,
    ctx: &egui::Context,
    show_sidebar: bool,
    render_content: F,
) where
    F: FnOnce(&mut MusicPlayerApp, &mut egui::Ui, &egui::Context),
{
    // Header (Top Navbar)
    egui::TopBottomPanel::top("header")
        .exact_height(50.0)
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(25, 25, 25)))
        .show(ctx, |ui| {
            render_header(app, ui);
        });

    // Footer (Player Controls) - Overlay at bottom without taking vertical space
    // NOTE: Player bar centering happens here with ui.vertical_centered()
    // Player controls centering: see src/ui_components/player.rs render_all_controls()
    if app.current_track_id.is_some() {
        let screen_rect = ctx.content_rect();
        let footer_height = 53.0;
        
        egui::Area::new(egui::Id::new("player_overlay"))
            .fixed_pos(egui::pos2(0.0, screen_rect.max.y - footer_height))
            .show(ctx, |ui| {
                ui.set_width(screen_rect.width());
                ui.set_height(footer_height);
                
                // Background
                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    egui::Color32::from_rgb(25, 25, 25),
                );
                
                // Use full width for footer controls
                ui.add_space(6.0);
                render_player(app, ui);
            });
    }

    // Sidebar (conditional and collapsible)
    if show_sidebar {
        if !app.queue_collapsed {
            // Full sidebar with queue
            egui::SidePanel::left("sidebar")
                .exact_width(400.0)
                .resizable(false)
                .frame(egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(15, 15, 15))
                    .inner_margin(egui::Margin::symmetric(10, 10))
                )
                .show(ctx, |ui| {
                    if let Some(clicked_queue_idx) = crate::ui_components::playlist_sidebar::render_playlist_tracks(app, ui, ctx) {
                        // clicked_queue_idx is now a position in the queue, get the actual track
                        if let Some(&original_idx) = app.playback_queue.current_queue.get(clicked_queue_idx) {
                            if let Some(track_id) = app.playback_queue.original_tracks.get(original_idx).map(|t| t.id) {
                                app.play_track(track_id);
                            }
                        }
                    }
                });
        } else {
            // Collapsed sidebar - just a thin bar with expand button
            egui::SidePanel::left("sidebar_collapsed")
                .exact_width(50.0)
                .resizable(false)
                .frame(egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(15, 15, 15))
                )
                .show(ctx, |ui| {
                    ui.add_space(10.0);
                    ui.vertical_centered(|ui| {
                        // Expand button
                        if ui.add_sized([40.0, 40.0], egui::Button::new("▶").fill(egui::Color32::from_rgb(45, 45, 50))).clicked() {
                            app.queue_collapsed = false;
                        }
                    });
                });
        }
    }

    // Central Panel (Content)
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE
            .fill(egui::Color32::BLACK)
        )
        .show(ctx, |ui| {
            // Add bottom padding if footer is visible (53px footer + 20px extra space)
            let bottom_padding = if app.current_track_id.is_some() { 73.0 } else { 0.0 };
            
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ctx.request_repaint();
                    render_content(app, ui, ctx);
                    ui.add_space(bottom_padding);
                });
        });
}

/// Load no_artwork.png texture from assets
fn load_no_artwork_texture(app: &mut MusicPlayerApp, ctx: &egui::Context) {
    let artwork_bytes = include_bytes!("../assets/no_artwork.png");
    if let Ok(image) = image::load_from_memory(artwork_bytes) {
        let rgba = image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        app.no_artwork_texture = Some(ctx.load_texture("no_artwork", color_image, egui::TextureOptions::LINEAR));
        log::info!("[Layout] Loaded no_artwork.png texture");
    } else {
        log::error!("[Layout] Failed to load no_artwork.png");
    }
}
