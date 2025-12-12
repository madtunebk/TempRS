use eframe::egui::{self, Color32, CornerRadius};
use crate::app::player_app::{MusicPlayerApp, SearchType};
use crate::utils::artwork::load_thumbnail_artwork;
use crate::ui_components::colors::*;
use std::sync::mpsc::channel;

mod tracks;
mod playlists;

/// Main search view dispatcher
pub fn render_search_view(app: &mut MusicPlayerApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.vertical(|ui| {
        ui.add_space(20.0);

        // Loading overlay
        if app.content.search_loading {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("Searching...")
                        .size(16.0)
                        .color(Color32::GRAY),
                );
            });
            return;
        }
        
        // Calculate total items
        let total_items = match app.content.search_type {
            SearchType::Tracks => app.content.search_results_tracks.len(),
            SearchType::Playlists => app.content.search_results_playlists.len(),
        };
        
        // Empty state when no results
        if total_items == 0 && !app.content.search_query.is_empty() {
            render_empty_state(ui);
            return;
        }
        
        // Header with result count
        if total_items > 0 {
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                
                let type_name = match app.content.search_type {
                    SearchType::Tracks => "Tracks",
                    SearchType::Playlists => "Playlists",
                };
                
                ui.label(
                    egui::RichText::new(format!("🔍 Search Results: {} ({} {})", 
                        app.content.search_query,
                        total_items,
                        if total_items == 1 { 
                            type_name.trim_end_matches('s') 
                        } else { 
                            type_name 
                        }
                    ))
                    .size(24.0)
                    .color(egui::Color32::WHITE)
                    .strong()
                );
            });
            
            ui.add_space(20.0);
        }
        
        // OPTIMIZATION: Preload artwork for visible items when results first load
        preload_visible_artwork(app, ctx);

        // Results grid with pagination
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Calculate total items for pagination
                let total_items = match app.content.search_type {
                    SearchType::Tracks => app.content.search_results_tracks.len(),
                    SearchType::Playlists => app.content.search_results_playlists.len(),
                };
                
                match app.content.search_type {
                    SearchType::Tracks => tracks::render_tracks_grid_paginated(app, ui, ctx),
                    SearchType::Playlists => playlists::render_playlists_grid_paginated(app, ui, ctx),
                }

                // Pagination controls (centered)
                if total_items > 0 {
                    ui.add_space(30.0);
                    
                    crate::ui_components::helpers::render_pagination_controls(
                        ui,
                        &mut app.content.search_page,
                        total_items,
                        app.content.search_page_size,
                    );
                    
                    ui.add_space(20.0);
                }
            });
    });
    
    // Note: Background task checking now handled in player_app.rs::update()
}

/// Preload artwork for first batch of visible results for instant display
fn preload_visible_artwork(app: &mut MusicPlayerApp, ctx: &egui::Context) {
    // Collect track IDs and URLs to avoid borrow checker issues
    let artwork_data: Vec<(u64, String)> = match app.content.search_type {
        SearchType::Tracks => {
            app.content.search_results_tracks
                .iter()
                .take(20)
                .filter_map(|track| {
                    track.artwork_url.as_ref().map(|url| {
                        (track.id, url.replace("-large.jpg", "-t500x500.jpg"))
                    })
                })
                .filter(|(_, url)| {
                    !app.ui.thumb_cache.contains_key(url)
                        && !app.ui.thumb_pending.contains_key(url)
                })
                .collect()
        }
        SearchType::Playlists => {
            app.content.search_results_playlists
                .iter()
                .take(20)
                .filter_map(|playlist| {
                    // For playlists, use playlist ID
                    playlist.artwork_url.as_ref().map(|url| {
                        (playlist.id, url.replace("-large.jpg", "-t500x500.jpg"))
                    })
                })
                .filter(|(_, url)| {
                    !app.ui.thumb_cache.contains_key(url)
                        && !app.ui.thumb_pending.contains_key(url)
                })
                .collect()
        }
    };

    // Now load all collected artwork using IDs for caching
    for (id, url) in artwork_data {
        load_thumbnail_artwork(app, ctx, id, url, false);
    }
}

/// Public function to trigger search from header
pub fn trigger_search(app: &mut MusicPlayerApp) {
    // Automatically switch to Search tab when searching
    app.ui.selected_tab = crate::app::player_app::MainTab::Search;
    perform_search(app);
}

/// Shared helper: Draw placeholder when no artwork available
pub(crate) fn draw_placeholder(ui: &mut egui::Ui, rect: egui::Rect) {
    ui.painter()
        .rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(35, 35, 40));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "♪",
        egui::FontId::proportional(50.0),
        Color32::from_rgb(60, 60, 65),
    );
}

/// Shared helper: Draw no_artwork.png or fallback to placeholder
pub(crate) fn draw_no_artwork(app: &MusicPlayerApp, ui: &mut egui::Ui, rect: egui::Rect) {
    if let Some(no_artwork) = &app.ui.no_artwork_texture {
        ui.painter().image(
            no_artwork.id(),
            rect,
            egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(1.0, 1.0),
            ),
            Color32::WHITE,
        );
    } else {
        draw_placeholder(ui, rect);
    }
}

fn perform_search(app: &mut MusicPlayerApp) {
    if app.content.search_query.trim().is_empty() {
        return;
    }

    app.content.search_loading = true;
    app.content.search_results_tracks.clear();
    app.content.search_results_playlists.clear();
    app.content.search_next_href = None;
    app.content.search_has_more = false;
    app.content.search_page = 0;  // Reset to first page

    let query = app.content.search_query.clone();
    let search_type = app.content.search_type;
    let token = match app.content.app_state.get_token() {
        Some(t) => t,
        None => return,
    };

    let (tx, rx) = channel();
    app.tasks.search_rx = Some(rx);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            match search_type {
                SearchType::Tracks => {
                    // Smart search: fetch until we have ~18 playable tracks
                    match crate::app::playlists::search_tracks_smart(&token, &query, 18).await {
                        Ok(response) => {
                            let _ = tx.send(
                                crate::app::player_app::SearchResults {
                                    tracks: response.collection,
                                    playlists: Vec::new(),
                                    next_href: response.next_href,
                                },
                            );
                        }
                        Err(e) => {
                            log::error!("[Search] Failed: {}", e);
                        }
                    }
                }
                SearchType::Playlists => {
                    match crate::app::playlists::search_playlists_paginated(&token, &query, 18)
                        .await
                    {
                        Ok(response) => {
                            let _ = tx.send(
                                crate::app::player_app::SearchResults {
                                    tracks: Vec::new(),
                                    playlists: response.collection,
                                    next_href: response.next_href,
                                },
                            );
                        }
                        Err(e) => {
                            log::error!("[Search] Failed: {}", e);
                        }
                    }
                }
            }
        });
    });
}

/// Render empty state with suggestions
fn render_empty_state(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);
        
        // Icon
        ui.label(
            egui::RichText::new("⌕")
                .size(64.0)
                .color(TEXT_TERTIARY)
        );
        
        ui.add_space(20.0);
        
        // Message
        ui.label(
            egui::RichText::new("No results found")
                .size(18.0)
                .color(TEXT_PRIMARY)
        );
        
        ui.add_space(10.0);
        
        // Suggestions
        ui.label(
            egui::RichText::new("Try searching for:")
                .size(14.0)
                .color(TEXT_SECONDARY)
        );
        
        ui.add_space(15.0);
        
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 200.0);
            
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("• Artist names").size(13.0).color(TEXT_SECONDARY));
                ui.label(egui::RichText::new("• Track titles").size(13.0).color(TEXT_SECONDARY));
                ui.label(egui::RichText::new("• Playlist names").size(13.0).color(TEXT_SECONDARY));
                ui.label(egui::RichText::new("• Genres (e.g., electronic, hip-hop)").size(13.0).color(TEXT_SECONDARY));
            });
        });
    });
}

