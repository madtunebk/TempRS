use crate::app::player_app::MusicPlayerApp;
use crate::app::playlists::Track;
use crate::ui_components::helpers::{calculate_grid_layout, render_track_card};
use eframe::egui::{self, Color32};

/// Action to take when interacting with suggestions track grid
#[derive(Debug, Clone, Copy)]
enum SuggestionsAction {
    PlaySingle(u64), // Play single track by ID
    PlayAsPlaylist,  // Load all as playlist
}

/// Suggestions view - Shows personalized recommendations in grid layout with pagination
pub fn render_suggestions_view(app: &mut MusicPlayerApp, ui: &mut egui::Ui) {
    // Check for background fetch completion first
    app.check_suggestions_updates();

    // Refresh suggestions when opening the tab (once per visit)
    if !app.content.suggestions_initial_fetch_done && !app.content.suggestions_loading {
        app.fetch_all_suggestions();
        app.content.suggestions_initial_fetch_done = true;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(20.0);

        // Title with track count
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("✨ Suggestions for You")
                    .size(24.0)
                    .color(egui::Color32::WHITE)
                    .strong(),
            );

            if !app.content.suggestions_tracks.is_empty() {
                ui.add_space(15.0);
                ui.label(
                    egui::RichText::new(format!(
                        "({} tracks)",
                        app.content.suggestions_tracks.len()
                    ))
                    .size(16.0)
                    .color(egui::Color32::GRAY),
                );
            }
        });

        ui.add_space(20.0);

        // Show loading state
        if app.content.suggestions_loading && app.content.suggestions_tracks.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("Loading personalized suggestions...")
                        .size(16.0)
                        .color(Color32::GRAY),
                );
            });
            return;
        }

        // Show empty state if no suggestions
        if app.content.suggestions_tracks.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.label(egui::RichText::new("✨").size(64.0).color(Color32::GRAY));
                ui.add_space(15.0);
                ui.label(
                    egui::RichText::new("No suggestions yet")
                        .size(20.0)
                        .color(Color32::GRAY),
                );
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(
                        "Start listening to music to discover personalized recommendations",
                    )
                    .size(14.0)
                    .color(Color32::DARK_GRAY),
                );
            });
            return;
        }

        // Apply filter
        let mut filtered_tracks = app.content.suggestions_tracks.clone();
        let filter_text = app.content.suggestions_search_filter.to_lowercase();
        if !filter_text.is_empty() {
            filtered_tracks.retain(|track| {
                track.title.to_lowercase().contains(&filter_text)
                    || track.user.username.to_lowercase().contains(&filter_text)
                    || track
                        .genre
                        .as_ref()
                        .is_some_and(|g| g.to_lowercase().contains(&filter_text))
            });
        }

        // Apply sorting
        match app.content.suggestions_sort_order {
            crate::app::player_app::SuggestionsSortOrder::Default => {
                // Keep API order
            }
            crate::app::player_app::SuggestionsSortOrder::TitleAZ => {
                filtered_tracks.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            }
            crate::app::player_app::SuggestionsSortOrder::ArtistAZ => {
                filtered_tracks.sort_by(|a, b| {
                    a.user
                        .username
                        .to_lowercase()
                        .cmp(&b.user.username.to_lowercase())
                });
            }
        }

        let total_suggestions = filtered_tracks.len();

        // Calculate pagination
        let start_idx = app.content.suggestions_page * app.content.suggestions_page_size;
        let end_idx = (start_idx + app.content.suggestions_page_size).min(total_suggestions);
        let page_tracks: Vec<_> = filtered_tracks[start_idx..end_idx].to_vec();

        // Preload artwork for visible tracks
        preload_suggestions_artwork(app, ui.ctx(), &page_tracks);

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
                egui::TextEdit::singleline(&mut app.content.suggestions_search_filter)
                    .hint_text("Filter by title, artist, or genre...")
                    .desired_width(300.0),
            );

            // Reset to page 0 when filter changes
            if search_response.changed() {
                app.content.suggestions_page = 0;
            }

            // Clear button
            if !app.content.suggestions_search_filter.is_empty() {
                ui.add_space(5.0);
                if ui.button("✖").clicked() {
                    app.content.suggestions_search_filter.clear();
                    app.content.suggestions_page = 0;
                }
            }

            ui.add_space(20.0);

            // Sort dropdown
            ui.label(
                egui::RichText::new("Sort:")
                    .size(14.0)
                    .color(egui::Color32::GRAY),
            );
            ui.add_space(5.0);

            let sort_response = egui::ComboBox::from_id_salt("suggestions_sort")
                .selected_text(match app.content.suggestions_sort_order {
                    crate::app::player_app::SuggestionsSortOrder::Default => "Default",
                    crate::app::player_app::SuggestionsSortOrder::TitleAZ => "Title (A-Z)",
                    crate::app::player_app::SuggestionsSortOrder::ArtistAZ => "Artist (A-Z)",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut app.content.suggestions_sort_order,
                        crate::app::player_app::SuggestionsSortOrder::Default,
                        "Default",
                    );
                    ui.selectable_value(
                        &mut app.content.suggestions_sort_order,
                        crate::app::player_app::SuggestionsSortOrder::TitleAZ,
                        "Title (A-Z)",
                    );
                    ui.selectable_value(
                        &mut app.content.suggestions_sort_order,
                        crate::app::player_app::SuggestionsSortOrder::ArtistAZ,
                        "Artist (A-Z)",
                    );
                });

            // Reset to page 0 when sort changes
            if sort_response.response.changed() {
                app.content.suggestions_page = 0;
            }
        });

        ui.add_space(15.0);

        // Render tracks grid (current page only)
        if let Some(action) = render_suggestions_grid(app, ui, &page_tracks) {
            match action {
                SuggestionsAction::PlaySingle(track_id) => {
                    log::info!("[Suggestions] Playing single track: {}", track_id);
                    if let Some(track) = filtered_tracks.iter().find(|t| t.id == track_id) {
                        app.audio.playback_queue.load_tracks(vec![track.clone()]);
                        app.play_track(track_id);
                    }
                }
                SuggestionsAction::PlayAsPlaylist => {
                    log::info!(
                        "[Suggestions] Loading all {} suggestions as playlist",
                        filtered_tracks.len()
                    );
                    app.audio
                        .playback_queue
                        .load_tracks(filtered_tracks.clone());
                    if let Some(first_track) = app.audio.playback_queue.current_track() {
                        app.play_track(first_track.id);
                    }
                }
            }
        }

        ui.add_space(30.0);

        // Pagination controls (centered, same as Likes/History)
        crate::ui_components::helpers::render_pagination_controls(
            ui,
            &mut app.content.suggestions_page,
            total_suggestions,
            app.content.suggestions_page_size,
        );

        ui.add_space(20.0);
    });
}

/// Render suggestions tracks grid (returns action if any)
fn render_suggestions_grid(
    app: &mut MusicPlayerApp,
    ui: &mut egui::Ui,
    tracks: &[Track],
) -> Option<SuggestionsAction> {
    let (items_per_row, padding) = calculate_grid_layout(ui.available_width(), 220.0, 15.0);

    let mut action = None;

    for chunk in tracks.chunks(items_per_row) {
        ui.horizontal(|ui| {
            ui.add_space(padding);
            for track in chunk {
                let (clicked, shift_clicked, _right_clicked) =
                    render_track_card(app, ui, track, 220.0);
                if clicked {
                    action = Some(SuggestionsAction::PlaySingle(track.id));
                } else if shift_clicked {
                    action = Some(SuggestionsAction::PlayAsPlaylist);
                }
                ui.add_space(15.0);
            }
        });
        ui.add_space(15.0);
    }

    action
}

/// Preload artwork for visible suggestions tracks
fn preload_suggestions_artwork(app: &mut MusicPlayerApp, ctx: &egui::Context, tracks: &[Track]) {
    // Use same system as Search - check artwork_url in memory cache
    for track in tracks.iter() {
        let artwork_url = track
            .artwork_url
            .as_ref()
            .map(|url| url.replace("-large.jpg", "-t500x500.jpg"))
            .unwrap_or_default();

        if !artwork_url.is_empty() && !app.ui.thumb_cache.contains_key(&artwork_url) {
            // load_thumbnail_artwork handles disk cache check and async download
            crate::utils::artwork::load_thumbnail_artwork(app, ctx, track.id, artwork_url, false);
        }
    }
}
