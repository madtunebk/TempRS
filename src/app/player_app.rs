use crate::app::home::HomeContent;
use crate::app_state::{AppState, RepeatMode};
use crate::utils::oauth::OAuthManager;
use eframe::egui;
use log::{debug, error, info, warn};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

// State modules
use crate::state::{AudioState, AuthState, BackgroundTasks, ContentState, UIState};

// Constants
use crate::constants::*;

// Re-export enums from state modules for convenience
pub use crate::state::background_tasks::SearchResults;
pub use crate::state::content_state::{
    LikesSortOrder, PlaylistsSortOrder, SearchType, SuggestionsSortOrder,
};
pub use crate::state::ui_state::{AppScreen, MainTab};

#[allow(dead_code)]
pub struct MusicPlayerApp {
    // Audio state (playback, track info, FFT analysis, controls)
    pub audio: AudioState,

    // Auth state (OAuth, user info, token validation)
    pub auth: AuthState,

    // UI state (navigation, visuals, artwork, shaders, controls, splash)
    pub ui: UIState,

    // Content state (search, playlists, home, suggestions, likes, history)
    pub content: ContentState,

    // Background tasks (receivers for async operations)
    pub tasks: BackgroundTasks,
}

impl Default for MusicPlayerApp {
    fn default() -> Self {
        // Cache cleanup disabled - was causing app freezing on startup
        // TODO: Re-enable with better async implementation later

        let app_state = AppState::new();
        let oauth_manager = Self::create_oauth_manager();

        Self {
            audio: Self::create_audio_state(&app_state),
            auth: Self::create_auth_state(oauth_manager),
            ui: Self::create_ui_state(),
            content: Self::create_content_state(app_state),
            tasks: BackgroundTasks::default(),
        }
    }
}

impl MusicPlayerApp {
    /// Create OAuth manager with credentials from main.rs
    fn create_oauth_manager() -> OAuthManager {
        use crate::utils::oauth::OAuthConfig;

        let client_id = crate::SOUNDCLOUD_CLIENT_ID.to_string();
        let client_secret = crate::SOUNDCLOUD_CLIENT_SECRET.to_string();
        let redirect_uri = OAUTH_REDIRECT_URI.to_string();
        let config = OAuthConfig::new(client_id, client_secret, redirect_uri);

        OAuthManager::new(config)
    }

    /// Create AudioState with saved playback preferences and FFT based on renderer
    fn create_audio_state(app_state: &AppState) -> AudioState {
        // Check if we're using GPU renderer (FFT needed for shaders)
        let enable_fft = app_state.get_renderer_type() == crate::app_state::RendererType::Gpu;

        let mut audio = AudioState::new(enable_fft);
        audio.volume = app_state.get_volume();
        audio.muted = app_state.is_muted();
        audio.volume_before_mute = if audio.muted {
            audio.volume
        } else {
            DEFAULT_VOLUME_BEFORE_MUTE
        };
        audio.shuffle_mode = app_state.get_shuffle_mode();
        audio.repeat_mode = app_state.get_repeat_mode();
        audio
    }

    /// Create AuthState with OAuth manager
    #[allow(clippy::field_reassign_with_default)]
    fn create_auth_state(oauth_manager: OAuthManager) -> AuthState {
        let mut auth = AuthState::default();
        auth.oauth_manager = Some(oauth_manager);
        auth.token_check_interval = Duration::from_secs(TOKEN_CHECK_INTERVAL_SECS);
        auth
    }

    /// Create UIState with custom splash duration and artwork colors
    #[allow(clippy::field_reassign_with_default)]
    fn create_ui_state() -> UIState {
        let mut ui = UIState::default();
        ui.splash_min_duration = Duration::from_secs(SPLASH_MIN_DURATION_SECS);
        let (r, g, b) = DOMINANT_COLOR_RGB;
        ui.artwork_dominant_color = egui::Color32::from_rgb(r, g, b);
        ui.artwork_edge_colors = [
            egui::Color32::from_rgb(r, g, b),
            egui::Color32::from_rgb(r, g, b),
            egui::Color32::from_rgb(r, g, b),
            egui::Color32::from_rgb(r, g, b),
        ];
        ui
    }

    /// Create ContentState with app state and custom page sizes (grid layout)
    #[allow(clippy::field_reassign_with_default)]
    fn create_content_state(app_state: AppState) -> ContentState {
        let mut content = ContentState::default();
        content.app_state = app_state;
        content.search_page_size = GRID_PAGE_SIZE;
        content.suggestions_page_size = GRID_PAGE_SIZE;
        content.likes_page_size = GRID_PAGE_SIZE;
        content.playlists_page_size = GRID_PAGE_SIZE;
        content.history_page_size = GRID_PAGE_SIZE;
        content.home_content = HomeContent::new();
        content
    }

    /// Create a new MusicPlayerApp with shader initialized from eframe CreationContext
    pub fn new(cc: &eframe::CreationContext<'_>, use_gpu: bool) -> Self {
        // Store renderer type in a new app_state
        let app_state = AppState::new();
        let renderer_type = if use_gpu {
            crate::app_state::RendererType::Gpu
        } else {
            crate::app_state::RendererType::Cpu
        };
        app_state.set_renderer_type(renderer_type);

        let oauth_manager = Self::create_oauth_manager();

        // Now create app with properly configured audio state (FFT enabled/disabled based on renderer)
        let mut app = Self {
            audio: Self::create_audio_state(&app_state),
            auth: Self::create_auth_state(oauth_manager),
            ui: Self::create_ui_state(),
            content: Self::create_content_state(app_state),
            tasks: BackgroundTasks::default(),
        };

        // Initialize shaders only in GPU mode (WGPU available)
        if use_gpu {
            app.ui
                .shader_manager
                .initialize(cc.wgpu_render_state.as_ref());
        }

        app
    }

    /// Save playback configuration to app state
    pub fn save_playback_config(&self) {
        self.content.app_state.set_volume(self.audio.volume);
        self.content.app_state.set_muted(self.audio.muted);
        self.content
            .app_state
            .set_shuffle_mode(self.audio.shuffle_mode);
        self.content
            .app_state
            .set_repeat_mode(self.audio.repeat_mode);
    }

    /// Request artwork fetch in background
    pub fn request_artwork_fetch(&mut self, track_id: u64, artwork_url: &str) {
        if artwork_url.is_empty() {
            return;
        }

        // Check cache first using track ID for immediate display
        if let Some(cached_bytes) = crate::utils::cache::load_artwork_cache(track_id) {
            if let Ok(img) = crate::utils::artwork::load_artwork_from_bytes(&cached_bytes) {
                let (tx, rx) = channel::<egui::ColorImage>();
                self.tasks.artwork_rx = Some(rx);
                let _ = tx.send(img);
                self.ui.artwork_loading = false;
                return;
            }
        }

        self.ui.artwork_loading = true;
        self.ui.artwork_texture = None;

        let (_cancel_tx, rx) =
            crate::utils::artwork::fetch_artwork(track_id, artwork_url.to_string());
        self.tasks.artwork_rx = Some(rx);
    }

    /// Check for received artwork from background thread
    pub fn check_artwork(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.tasks.artwork_rx {
            if let Ok(img) = rx.try_recv() {
                let on_now_playing = matches!(self.ui.selected_tab, MainTab::NowPlaying);
                let is_gpu = self.content.app_state.get_renderer_type()
                    == crate::app_state::RendererType::Gpu;

                // Extract colors only if needed for visuals (GPU + Now Playing visible)
                if is_gpu && on_now_playing {
                    self.ui.artwork_dominant_color =
                        crate::utils::artwork::extract_dominant_color(&img);
                    self.ui.artwork_edge_colors =
                        crate::utils::artwork::extract_edge_colors(&img);
                }

                if on_now_playing {
                    // Create texture immediately for visible view
                    self.ui.artwork_texture = Some(ctx.load_texture(
                        "artwork",
                        img,
                        egui::TextureOptions::LINEAR,
                    ));
                } else {
                    // Defer texture upload until Now Playing is shown
                    self.ui.pending_artwork_image = Some(img);
                    self.ui.artwork_texture = None;
                }

                self.ui.artwork_loading = false;
                self.tasks.artwork_rx = None;
            }
        }
    }

    /// Play a track by ID
    pub fn play_track(&mut self, track_id: u64) {
        info!(
            "[PLAY] play_track({}) called - is_playing={}, current_track_id={:?}",
            track_id, self.audio.is_playing, self.audio.current_track_id
        );

        // Increment session to invalidate any pending async operations
        self.audio.playback_session = self.audio.playback_session.wrapping_add(1);
        log::debug!(
            "[Session] New playback session: {}",
            self.audio.playback_session
        );

        // Don't send stop command - the audio controller will replace the old player automatically
        // This prevents interrupting the download of new track
        self.audio.is_playing = false; // Temporarily set to false, will be set to true when playback starts

        // Clear previous errors
        self.ui.last_playback_error = None;

        // Note: Token validity is checked by periodic check_token_expiry() which runs every 60s
        // and automatically refreshes before expiry. No need to check here.

        // Update queue position to the selected track
        self.audio.playback_queue.jump_to_track_id(track_id);

        // Get track from queue (which has the current tracks loaded)
        let track = match self.audio.playback_queue.current_track() {
            Some(t) => t.clone(),
            None => {
                let error_msg = format!("Track {} not found in queue", track_id);
                warn!("{}", error_msg);
                self.ui.last_playback_error = Some(error_msg);
                return;
            }
        };

        // Check if track is streamable but missing stream_url (database track)
        // If so, fetch it on-demand instead of using is_track_playable check
        if track.streamable.unwrap_or(false) && track.stream_url.is_none() {
            log::info!("[PLAY] Database track detected, fetching stream URL on-demand");
            self.fetch_and_play_track(track_id);
            return;
        }

        // Validate track is playable (has stream_url)
        if !crate::utils::track_filter::is_track_playable(&track) {
            let error_msg = format!(
                "Track '{}' is not playable (geo-blocked or preview-only)",
                track.title
            );
            log::warn!("{}", error_msg);
            self.ui.last_playback_error = Some(error_msg);

            // Auto-skip to next track instead of stopping playback
            log::info!("[PLAY] Auto-skipping to next track...");
            self.play_next();
            return;
        }

        // Clone data we need
        let artwork_url = track.artwork_url.clone();

        // Update current track info
        self.audio.current_track_id = Some(track.id);
        self.audio.current_title = track.title.clone();
        self.audio.current_artist = track.user.username.clone();
        self.audio.current_genre = track.genre.clone();

        // Use full_duration if available (for long tracks), otherwise duration
        let actual_duration = track.full_duration.unwrap_or(track.duration);
        self.audio.current_duration_ms = actual_duration;
        self.audio.current_stream_url = track.stream_url.clone();
        self.audio.current_permalink_url = track.permalink_url.clone();

        // Debug logging for duration (especially for long tracks)
        if track.full_duration.is_some() && track.full_duration != Some(track.duration) {
            log::warn!("[Track] Duration mismatch - duration: {}ms, full_duration: {}ms (using full_duration)", 
                track.duration, track.full_duration.unwrap());
        }
        let duration_minutes = actual_duration / 1000 / 60;
        log::info!(
            "[Track] Duration from API: {}ms ({} minutes, {} seconds)",
            actual_duration,
            duration_minutes,
            (actual_duration / 1000) % 60
        );

        // Fetch artwork if available, otherwise clear old artwork
        if let Some(url) = artwork_url {
            self.request_artwork_fetch(track.id, &url);
        } else {
            // No artwork for this track - clear previous artwork
            self.ui.artwork_texture = None;
        }

        // Start playback if we have a stream URL
        if let (Some(stream_url), Some(oauth)) =
            (&self.audio.current_stream_url, &self.auth.oauth_manager)
        {
            if let Some(token) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                // Detect history DB tracks: they typically have full_duration=None and were fetched on-demand
                let is_history_track =
                    track.full_duration.is_none() && track.permalink_url.is_none();
                log::info!(
                    "Playing: {} by {} (duration: {}ms, history: {})",
                    self.audio.current_title,
                    self.audio.current_artist,
                    self.audio.current_duration_ms,
                    is_history_track
                );
                // Use prefetched CDN URL when valid to skip redirect delays
                let prefetched = if self.audio.has_valid_prefetch(track.id) {
                    self.audio.prefetch_cdn_url.clone()
                } else {
                    None
                };
                self.audio.audio_controller.play(
                    stream_url.clone(),
                    token.access_token.clone(),
                    track.id,
                    self.audio.current_duration_ms,
                    is_history_track,
                    prefetched,
                );
                self.audio.is_playing = true;
                log::info!(
                    "[PLAY] Playback started - is_playing={}",
                    self.audio.is_playing
                );
                self.audio.track_start_time = Some(Instant::now());

                // Reset the track finished flag for the new track
                self.audio.track_finished_handled = false;

                // Validate duration sync after a short delay (allow player to initialize)
                // This will be checked in the update loop

                // Reset prefetch trigger for the new track
                self.audio.prefetch_triggered = false;

                // Record this track to playback history (only when actually played)
                crate::app::queue::record_track_to_history(&track);

                // Refresh Home screen to show newly played track
                self.refresh_home_recently_played();
            } else {
                let error_msg = "Failed to get authentication token";
                error!("{}", error_msg);
                self.ui.last_playback_error = Some(error_msg.to_string());
            }
        } else {
            let error_msg = if self.audio.current_stream_url.is_none() {
                format!(
                    "Track '{}' has no stream URL (not streamable)",
                    self.audio.current_title
                )
            } else {
                "Authentication required".to_string()
            };
            error!("{}", error_msg);
            self.ui.last_playback_error = Some(error_msg);
        }
    }

    /// Toggle play/pause
    pub fn toggle_playback(&mut self) {
        log::info!(
            "[TOGGLE] toggle_playback called - is_playing={}, has_track={}",
            self.audio.is_playing,
            self.audio.current_track_id.is_some()
        );

        // Don't do anything if no track is loaded
        if self.audio.current_track_id.is_none() {
            log::warn!("[TOGGLE] Ignoring toggle - no track loaded");
            return;
        }

        if self.audio.is_playing {
            log::info!("[TOGGLE] Pausing playback");
            self.audio.audio_controller.pause();
            self.audio.is_playing = false;
        } else {
            // Check if track was stopped (track_start_time is None) or finished
            if self.audio.track_start_time.is_none() || self.audio.audio_controller.is_finished() {
                // Track was stopped or finished, restart from beginning
                if let Some(track_id) = self.audio.current_track_id {
                    log::info!("[TOGGLE] Track finished, restarting from beginning");
                    // Reset timing for restart
                    self.audio.track_start_time = Some(std::time::Instant::now());
                    self.play_track(track_id);
                }
            } else {
                // Normal resume from pause
                log::info!("[TOGGLE] Resuming playback");
                self.audio.audio_controller.resume();
                self.audio.is_playing = true;
            }
        }
    }

    /// Stop playback and reset state (ready to play another track)
    pub fn stop_playback(&mut self) {
        log::info!("[STOP] Stopping playback - clearing track state to hide player controls");

        // Increment session to invalidate any pending async operations
        self.audio.playback_session = self.audio.playback_session.wrapping_add(1);
        log::debug!(
            "[Session] Stop playback session: {}",
            self.audio.playback_session
        );

        self.audio.audio_controller.stop();
        self.audio.is_playing = false;
        self.ui.last_playback_error = None;
        // Clear track ID to hide player controls
        self.audio.current_track_id = None;
        // Reset track timing so it restarts from beginning
        self.audio.track_start_time = None;
    }

    /// Gracefully cleanup all resources before exit
    fn cleanup_and_exit(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        log::info!("[Shutdown] Starting graceful cleanup...");

        // 1. Stop audio playback and cleanup audio threads
        if self.audio.is_playing {
            log::info!("[Shutdown] Stopping audio playback...");
            self.audio.audio_controller.stop();
            self.audio.is_playing = false;
        }

        // Explicitly drop audio controller to free resources
        log::info!("[Shutdown] Releasing audio resources...");
        let _ = &mut self.audio.audio_controller;

        // 2. Save playback configuration
        log::info!("[Shutdown] Saving playback configuration...");
        self.save_playback_config();

        // 3. Clear all pending receivers to prevent thread leaks
        log::info!("[Shutdown] Clearing pending background tasks...");
        self.tasks.artwork_rx = None;
        self.tasks.user_avatar_rx = None;
        self.tasks.search_rx = None;
        self.tasks.playlist_rx = None;
        self.tasks.playlist_chunk_rx = None;
        self.tasks.home_recently_played_rx = None;
        self.tasks.home_recommendations_rx = None;
        self.tasks.track_fetch_rx = None;
        self.tasks.suggestions_rx = None;

        // 4. Clear texture caches
        log::info!("[Shutdown] Clearing texture caches...");
        self.ui.thumb_cache.clear();
        self.ui.artwork_texture = None;
        self.auth.user_avatar_texture = None;
        self.ui.no_artwork_texture = None;

        // 5. OAuth manager cleanup (tokens are already encrypted in DB)
        log::info!("[Shutdown] Cleaning up OAuth resources...");
        if self.auth.oauth_manager.is_some() {
            // OAuth tokens are persisted in encrypted database, safe to drop
            self.auth.oauth_manager = None;
        }

        // Shaders are managed by ShaderManager, will be cleaned up automatically

        log::info!("[Shutdown] Cleanup complete, closing application...");

        // Close the application
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Reset to clean state (no track loaded)
    #[allow(dead_code)]
    pub fn reset_player_state(&mut self) {
        info!("[RESET] Resetting player to clean state");
        info!(
            "[RESET] Before: is_playing={}, current_track_id={:?}",
            self.audio.is_playing, self.audio.current_track_id
        );
        self.audio.audio_controller.stop();
        self.audio.reset_track(); // Use AudioState helper method
        self.ui.last_playback_error = None; // Clear any error on reset
        self.ui.artwork_texture = None;
        log::info!(
            "[RESET] After: is_playing={}, current_track_id={:?}",
            self.audio.is_playing,
            self.audio.current_track_id
        );
    }

    /// Play next track in queue
    pub fn play_next(&mut self) {
        let next_track_id = self.audio.playback_queue.next().map(|t| t.id);

        if let Some(track_id) = next_track_id {
            self.play_track(track_id);
        } else if self.audio.repeat_mode == RepeatMode::All {
            // Loop back to start
            let first_track_id = self.audio.playback_queue.loop_to_start().map(|t| t.id);
            if let Some(track_id) = first_track_id {
                self.play_track(track_id);
            }
        }
    }

    /// Play previous track in queue
    pub fn play_previous(&mut self) {
        let prev_track_id = self.audio.playback_queue.previous().map(|t| t.id);

        if let Some(track_id) = prev_track_id {
            self.play_track(track_id);
        }
    }

    /// Toggle shuffle mode
    pub fn toggle_shuffle(&mut self) {
        self.audio.shuffle_mode = !self.audio.shuffle_mode;
        self.audio
            .playback_queue
            .set_shuffle(self.audio.shuffle_mode);
        self.save_playback_config();
        if self.audio.shuffle_mode {
            info!("Shuffle enabled");
        } else {
            info!("Shuffle disabled");
        }
    }

    /// Cycle repeat mode
    pub fn cycle_repeat_mode(&mut self) {
        self.audio.repeat_mode = match self.audio.repeat_mode {
            RepeatMode::None => {
                info!("Repeat All enabled");
                RepeatMode::All
            }
            RepeatMode::All => {
                info!("Repeat One enabled");
                // Disable shuffle when switching to Repeat One
                if self.audio.shuffle_mode {
                    self.audio.shuffle_mode = false;
                    self.audio.playback_queue.set_shuffle(false);
                    info!("Shuffle auto-disabled (incompatible with Repeat One)");
                }
                RepeatMode::One
            }
            RepeatMode::One => {
                info!("Repeat disabled");
                RepeatMode::None
            }
        };
        self.save_playback_config();
    }

    /// Set volume
    pub fn set_volume(&mut self, volume: f32) {
        self.audio.volume = volume.clamp(0.0, 1.0);
        self.audio.audio_controller.set_volume(self.audio.volume);
        self.save_playback_config();
    }

    /// Toggle mute
    pub fn toggle_mute(&mut self) {
        if self.audio.muted {
            self.audio.volume = self.audio.volume_before_mute;
            self.audio.muted = false;
        } else {
            self.audio.volume_before_mute = self.audio.volume;
            self.audio.volume = 0.0;
            self.audio.muted = true;
        }
        self.audio.audio_controller.set_volume(self.audio.volume);
        self.save_playback_config();
    }

    /// Seek to position
    pub fn seek_to(&mut self, position: Duration) {
        // Increment session to invalidate any pending async operations
        self.audio.playback_session = self.audio.playback_session.wrapping_add(1);
        log::debug!("[Session] Seek session: {}", self.audio.playback_session);

        // Reset prefetch trigger so it can run again after seek
        self.audio.prefetch_triggered = false;

        self.audio.audio_controller.seek(position);
        self.ui.is_seeking = true;
        self.ui.seek_target_pos = Some(position);
    }

    /// Get current playback position (legacy; progress bar uses throttled cache)
    #[allow(dead_code)]
    pub fn get_position(&self) -> Duration {
        self.audio.audio_controller.get_position()
    }

    /// Get track duration
    pub fn get_duration(&self) -> Duration {
        self.audio
            .audio_controller
            .get_duration()
            .unwrap_or(Duration::from_millis(self.audio.current_duration_ms))
    }

    /// Check if current track is liked
    pub fn is_current_track_liked(&self) -> bool {
        if let Some(track_id) = self.audio.current_track_id {
            self.content.liked_track_ids.contains(&track_id)
        } else {
            false
        }
    }

    /// Toggle like status of current track
    pub fn toggle_current_track_like(&mut self) {
        if let Some(track_id) = self.audio.current_track_id {
            self.toggle_like(track_id);
        } else {
            log::warn!("[Like] No track currently playing");
        }
    }

    /// Toggle like status for any track by ID
    pub fn toggle_like(&mut self, track_id: u64) {
        let result = crate::services::toggle_like(
            crate::services::LikeTarget::Track(track_id),
            &mut self.content.liked_track_ids,
            self.content.app_state.get_token(),
        );

        // Show appropriate toast based on result
        if let Some(_token) = self.content.app_state.get_token() {
            if result.is_liked {
                self.ui.toast_manager.show_success(&result.success_message);
            } else {
                self.ui.toast_manager.show_info(&result.success_message);
            }
        } else {
            self.ui.toast_manager.show_error(&result.error_message);
        }
    }

    /// Toggle like status for a playlist by ID
    pub fn toggle_playlist_like(&mut self, playlist_id: u64) {
        let result = crate::services::toggle_like(
            crate::services::LikeTarget::Playlist(playlist_id),
            &mut self.content.liked_playlist_ids,
            self.content.app_state.get_token(),
        );

        // Show appropriate toast based on result
        if let Some(_token) = self.content.app_state.get_token() {
            if result.is_liked {
                self.ui.toast_manager.show_success(&result.success_message);
            } else {
                self.ui.toast_manager.show_info(&result.success_message);
            }
        } else {
            self.ui.toast_manager.show_error(&result.error_message);
        }
    }

    /// Handle keyboard shortcuts (all require Ctrl modifier to avoid interfering with text input)
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Playback controls
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Space) {
                self.handle_play_pause_shortcut();
            }
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::L) {
                self.toggle_current_track_like();
            }

            // Playback mode controls
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S) {
                self.handle_shuffle_shortcut();
            }
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::R) {
                self.handle_repeat_shortcut();
            }

            // Volume controls
            if i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowUp) {
                self.handle_volume_up_shortcut();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowDown) {
                self.handle_volume_down_shortcut();
            }

            // Seek controls
            if i.modifiers.ctrl
                && i.key_pressed(egui::Key::ArrowRight)
                && self.audio.current_track_id.is_some()
            {
                self.handle_seek_forward_shortcut();
            }
            if i.modifiers.ctrl
                && i.key_pressed(egui::Key::ArrowLeft)
                && self.audio.current_track_id.is_some()
            {
                self.handle_seek_backward_shortcut();
            }
        });
    }

    /// Handle Ctrl+Space: Play/Pause
    fn handle_play_pause_shortcut(&mut self) {
        if self.audio.is_playing {
            self.audio.audio_controller.pause();
            self.audio.is_playing = false;
        } else if self.audio.current_track_id.is_some() {
            self.audio.audio_controller.resume();
            self.audio.is_playing = true;
        }
    }

    /// Handle Ctrl+Shift+S: Toggle shuffle
    fn handle_shuffle_shortcut(&mut self) {
        self.audio.shuffle_mode = !self.audio.shuffle_mode;
        self.audio
            .playback_queue
            .set_shuffle(self.audio.shuffle_mode);
        self.save_playback_config();
        let msg = if self.audio.shuffle_mode {
            "Shuffle on"
        } else {
            "Shuffle off"
        };
        self.ui.toast_manager.show_info(msg);
    }

    /// Handle Ctrl+Shift+R: Cycle repeat mode
    fn handle_repeat_shortcut(&mut self) {
        use crate::app_state::RepeatMode;
        self.audio.repeat_mode = match self.audio.repeat_mode {
            RepeatMode::None => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::None,
        };
        self.save_playback_config();
        let msg = match self.audio.repeat_mode {
            RepeatMode::None => "Repeat off",
            RepeatMode::All => "Repeat all",
            RepeatMode::One => "Repeat one",
        };
        self.ui.toast_manager.show_info(msg);
    }

    /// Handle Ctrl+Arrow Up: Volume up
    fn handle_volume_up_shortcut(&mut self) {
        let new_volume = (self.audio.volume + VOLUME_STEP).min(1.0);
        self.audio.volume = new_volume;
        self.audio.audio_controller.set_volume(new_volume);
        if self.audio.muted {
            self.audio.muted = false;
        }
    }

    /// Handle Ctrl+Arrow Down: Volume down
    fn handle_volume_down_shortcut(&mut self) {
        let new_volume = (self.audio.volume - VOLUME_STEP).max(0.0);
        self.audio.volume = new_volume;
        self.audio.audio_controller.set_volume(new_volume);
    }

    /// Handle Ctrl+Arrow Right: Seek forward
    fn handle_seek_forward_shortcut(&mut self) {
        let current_pos = self.audio.audio_controller.get_position();
        let new_pos = current_pos + Duration::from_secs(SEEK_STEP_SECS);
        if new_pos < Duration::from_millis(self.audio.current_duration_ms) {
            self.ui.seek_target_pos = Some(new_pos);
        }
    }

    /// Handle Ctrl+Arrow Left: Seek backward
    fn handle_seek_backward_shortcut(&mut self) {
        let current_pos = self.audio.audio_controller.get_position();
        let new_pos = current_pos.saturating_sub(Duration::from_secs(SEEK_STEP_SECS));
        self.ui.seek_target_pos = Some(new_pos);
    }

    /// Share current track (copy URL to clipboard)
    pub fn share_current_track(&mut self) {
        let success =
            crate::utils::clipboard::share_track_url(self.audio.current_permalink_url.as_deref());

        if success {
            self.ui
                .toast_manager
                .show_success("Track URL copied to clipboard!");
        } else {
            self.ui
                .toast_manager
                .show_error("Failed to copy URL - no track playing");
        }
    }

    /// Fetch user info (avatar and username) from /me endpoint
    pub fn fetch_user_info(&mut self) {
        if self.tasks.user_avatar_rx.is_some() {
            return; // Already fetching
        }

        // Use token helper to ensure fresh token
        let oauth = match &self.auth.oauth_manager {
            Some(o) => o.clone(),
            None => return,
        };

        let token = match crate::utils::token_helper::get_valid_token_sync(&oauth) {
            Some(t) => t.access_token,
            None => {
                log::warn!("[FetchUserInfo] No valid token available");
                return;
            }
        };

        let (tx, rx) = channel();
        self.tasks.user_avatar_rx = Some(rx);

        std::thread::spawn(move || {
            let rt = match crate::utils::error_handling::create_runtime() {
                Ok(r) => r,
                Err(e) => {
                    log::error!("[PlayerApp] {}", e);
                    return;
                }
            };
            rt.block_on(async {
                let client = crate::utils::http::client();

                if let Ok(resp) = client
                    .get("https://api.soundcloud.com/me")
                    .header("Authorization", format!("OAuth {}", token))
                    .send()
                    .await
                {
                    if let Ok(user_json) = resp.json::<serde_json::Value>().await {
                        debug!(
                            "Received user data: username={}, avatar={}",
                            user_json["username"].as_str().unwrap_or("N/A"),
                            user_json["avatar_url"].as_str().unwrap_or("N/A")
                        );

                        // Get avatar URL - use larger size if available
                        if let Some(avatar_url) = user_json["avatar_url"].as_str() {
                            // Replace size parameter to get larger avatar (t500x500 instead of default)
                            let large_avatar_url = if avatar_url.contains("-large.jpg") {
                                avatar_url.replace("-large.jpg", "-t500x500.jpg")
                            } else if avatar_url.contains("-t500x500.jpg") {
                                avatar_url.to_string()
                            } else {
                                // Handle other formats or default size
                                avatar_url.replace(".jpg", "-t500x500.jpg")
                            };

                            // Download avatar image
                            if let Ok(img_resp) = client.get(&large_avatar_url).send().await {
                                if let Ok(bytes) = img_resp.bytes().await {
                                    if let Ok(img) = image::load_from_memory(&bytes) {
                                        let rgba = img.to_rgba8();
                                        let size = [rgba.width() as usize, rgba.height() as usize];
                                        let pixels = rgba.as_flat_samples();
                                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                            size,
                                            pixels.as_slice(),
                                        );
                                        let _ = tx.send(color_image);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        });
    }

    /// Check for user avatar updates
    pub fn check_user_avatar(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.tasks.user_avatar_rx {
            if let Ok(color_image) = rx.try_recv() {
                self.auth.user_avatar_texture = Some(ctx.load_texture(
                    "user_avatar",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
                self.tasks.user_avatar_rx = None;
            }
        }
    }

    /// Check for playlist chunk updates (progressive loading)
    pub fn check_playlist_chunks(&mut self) {
        if let Some(rx) = &self.tasks.playlist_chunk_rx {
            if let Ok(chunk_tracks) = rx.try_recv() {
                if chunk_tracks.is_empty() {
                    self.handle_playlist_complete();
                } else {
                    self.handle_playlist_chunk(chunk_tracks);
                }
            }
        }
    }

    /// Handle playlist loading completion
    fn handle_playlist_complete(&mut self) {
        log::info!("[App] Playlist loading complete");
        self.tasks.playlist_chunk_rx = None;
        self.content.playlist_loading_id = None;
    }

    /// Handle incoming playlist chunk
    fn handle_playlist_chunk(&mut self, chunk_tracks: Vec<crate::app::playlists::Track>) {
        let chunk_size = chunk_tracks.len();
        log::info!("[App] Received chunk with {} tracks", chunk_size);

        let is_first_chunk = self.audio.playback_queue.original_tracks.is_empty();
        let playable_tracks = Self::filter_playable_tracks(chunk_tracks);

        if !playable_tracks.is_empty() {
            let filtered_count = playable_tracks.len();
            if filtered_count < chunk_size {
                log::info!(
                    "[App] Filtered {} → {} playable tracks",
                    chunk_size,
                    filtered_count
                );
            }

            self.audio
                .playback_queue
                .append_tracks(playable_tracks.clone());
            log::info!(
                "[App] Added {} tracks to queue (total: {})",
                filtered_count,
                self.audio.playback_queue.original_tracks.len()
            );

            if is_first_chunk {
                self.start_first_chunk_playback();
            }
        }
    }

    /// Filter out non-playable tracks (geo-blocked, preview-only, non-streamable)
    fn filter_playable_tracks(
        tracks: Vec<crate::app::playlists::Track>,
    ) -> Vec<crate::app::playlists::Track> {
        tracks
            .into_iter()
            .filter(|t| {
                if !t.streamable.unwrap_or(false) {
                    return false;
                }

                if let Some(policy) = &t.policy {
                    if policy.to_uppercase() == "BLOCK" {
                        log::debug!("[Chunk] Filtering geo-locked: {}", t.title);
                        return false;
                    }
                }

                if let Some(access) = &t.access {
                    let access_lower = access.to_lowercase();
                    if access_lower == "blocked" || access_lower == "preview" {
                        log::debug!("[Chunk] Filtering restricted access: {}", t.title);
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Start playback with first track from first chunk
    fn start_first_chunk_playback(&mut self) {
        if let Some(first_track) = self.audio.playback_queue.current_track() {
            let track_id = first_track.id;
            log::info!("[App] Starting playback with first chunk");
            self.play_track(track_id);
        }
    }

    /// Logout user
    pub fn logout(&mut self) {
        // Stop playback first
        self.audio.audio_controller.stop();

        // Use state module helper methods for cleanup
        self.auth.clear_session();
        self.content.reset_all_content();
        self.audio.reset_track();

        // Clear UI state
        self.ui.selected_tab = MainTab::Home;
        self.ui.screen = AppScreen::Splash;
        self.ui.artwork_texture = None;
        self.ui.artwork_loading = false;
        self.ui.thumb_cache.clear();
        self.ui.thumb_pending.clear();

        // Clear background tasks
        self.tasks.clear_all();

        // Clear token from app_state
        self.content.app_state.clear_token();

        // Shader manager retains shaders across logout - no need to reinitialize
    }

    /// Check playback progress and trigger prefetch at 70-80%
    pub fn check_prefetch_trigger(&mut self) {
        // Only prefetch if:
        // 1. Currently playing
        // 2. Not already triggered for this track
        // 3. Next track exists
        // 4. Progress is 70-80%

        if !self.audio.is_playing || self.audio.prefetch_triggered {
            return;
        }

        let position = self.audio.audio_controller.get_position();
        let duration = Duration::from_millis(self.audio.current_duration_ms);

        if duration.as_secs() == 0 {
            return; // Avoid division by zero
        }

        let progress = position.as_secs_f32() / duration.as_secs_f32();

        // Trigger between 70-80% (only once per track)
        if (0.70..=0.80).contains(&progress) {
            // Extract data we need before calling trigger_prefetch (to avoid borrow checker issues)
            if let Some(next_track) = self.audio.playback_queue.peek_next() {
                let track_id = next_track.id;
                let stream_url = next_track.stream_url.clone();
                self.trigger_prefetch(track_id, stream_url.as_deref());
            }
        }
    }

    /// Trigger prefetch for next track's CDN URL
    fn trigger_prefetch(&mut self, track_id: u64, stream_url: Option<&str>) {
        // Skip if already prefetched for this track
        if self.audio.has_valid_prefetch(track_id) {
            log::debug!("[Prefetch] Valid cache exists for track {}", track_id);
            return;
        }

        // Mark as triggered to prevent duplicate requests
        self.audio.prefetch_triggered = true;

        let stream_url = match stream_url {
            Some(url) => url.to_string(),
            None => {
                log::warn!(
                    "[Prefetch] Track {} has no stream_url, skipping prefetch",
                    track_id
                );
                return;
            }
        };

        let token = match &self.auth.oauth_manager {
            Some(oauth) => match crate::utils::token_helper::get_valid_token_sync(oauth) {
                Some(t) => t.access_token.clone(),
                None => {
                    log::warn!("[Prefetch] No valid token");
                    return;
                }
            },
            None => return,
        };

        log::info!(
            "[Prefetch] Starting prefetch for track {} at 70-80% progress",
            track_id
        );

        // Capture current session for validation in callback
        let session = self.audio.playback_session;

        let (tx, rx) = channel();
        self.tasks.prefetch_rx = Some(rx);

        // Spawn prefetch in background
        std::thread::spawn(move || {
            let rt = match crate::utils::error_handling::create_runtime() {
                Ok(r) => r,
                Err(e) => {
                    log::error!("[Prefetch] Failed to create runtime: {}", e);
                    return;
                }
            };
            rt.block_on(async {
                match crate::utils::stream_utils::prefetch_stream_url(&stream_url, &token).await {
                    Ok(cdn_url) => {
                        log::info!(
                            "[Prefetch] Successfully prefetched CDN URL for track {}",
                            track_id
                        );
                        let _ = tx.send((session, track_id, cdn_url));
                    }
                    Err(e) => {
                        log::warn!(
                            "[Prefetch] Failed to prefetch for track {}: {}",
                            track_id,
                            e
                        );
                    }
                }
            });
        });
    }

    /// Check for prefetch completion
    pub fn check_prefetch_updates(&mut self) {
        if let Some(rx) = &self.tasks.prefetch_rx {
            if let Ok((session, track_id, cdn_url)) = rx.try_recv() {
                // Guard: Only apply prefetch if session matches
                if session != self.audio.playback_session {
                    log::debug!(
                        "[Prefetch] Ignoring stale prefetch for track {} (session {} != {})",
                        track_id,
                        session,
                        self.audio.playback_session
                    );
                    self.tasks.prefetch_rx = None;
                    return;
                }

                log::info!("[Prefetch] Received CDN URL for track {}", track_id);
                self.audio.prefetch_cdn_url = Some(cdn_url);
                self.audio.prefetch_timestamp = Some(Instant::now());
                self.audio.prefetched_for_track_id = Some(track_id);
                self.tasks.prefetch_rx = None;
            }
        }
    }

    /// Check if track finished and handle auto-play
    pub fn check_track_finished(&mut self) {
        // Only check for track completion if we're currently playing (not paused)
        // This prevents false positives when sink is empty due to pause state
        if self.audio.is_playing && self.audio.audio_controller.is_finished() {
            // DEBOUNCE: If we already handled this track finish, don't handle it again
            // This prevents the event loop from triggering 30+ times per second
            if self.audio.track_finished_handled {
                return;
            }

            // Additional check: ensure we have a valid track and it's actually started
            // track_start_time is set after audio successfully loads, so this prevents
            // false positives during the loading phase
            if self.audio.current_track_id.is_none() || self.audio.track_start_time.is_none() {
                return;
            }

            // Prevent race condition: don't treat as finished if track just started
            if let Some(start_time) = self.audio.track_start_time {
                if start_time.elapsed() < Duration::from_secs(MIN_TRACK_ELAPSED_SECS) {
                    return;
                }
            }

            // Mark as handled BEFORE processing to prevent re-entry during the same track
            self.audio.track_finished_handled = true;

            log::info!("Track finished, handling auto-play/stop");

            match self.audio.repeat_mode {
                RepeatMode::One => {
                    // Replay current track
                    if let Some(track_id) = self.audio.current_track_id {
                        info!("Repeat One: replaying track {}", track_id);
                        self.play_track(track_id);
                    }
                }
                RepeatMode::All => {
                    // Check if we're at the end of the queue
                    let at_end = self
                        .audio
                        .playback_queue
                        .current_index
                        .map(|idx| idx >= self.audio.playback_queue.current_queue.len() - 1)
                        .unwrap_or(true);

                    if at_end {
                        // Loop back to first track
                        info!("Repeat All: looping back to first track");
                        if let Some(first_track) = self.audio.playback_queue.original_tracks.first()
                        {
                            self.audio.playback_queue.current_index = Some(0);
                            self.play_track(first_track.id);
                        }
                    } else {
                        self.play_next();
                    }
                }
                RepeatMode::None => {
                    // Just play next, stop if at end
                    let can_play_next = self
                        .audio
                        .playback_queue
                        .current_index
                        .map(|idx| idx < self.audio.playback_queue.current_queue.len() - 1)
                        .unwrap_or(false);

                    if can_play_next {
                        self.play_next();
                    } else {
                        // Check if this was single-track playback
                        let is_single_track = self.audio.playback_queue.current_queue.len() == 1;

                        if is_single_track {
                            // CRITICAL: Don't trigger fetch if one is already in progress
                            if self.tasks.track_fetch_rx.is_some() {
                                return; // Already fetching next track, wait for it
                            }

                            // Try to play random track from history (excluding current track)
                            info!("Single track finished, picking random track from history");

                            if let Some(current_id) = self.audio.current_track_id {
                                match crate::utils::playback_history::PlaybackHistoryDB::new() {
                                    Ok(db) => {
                                        // Fetch recent tracks (we'll filter out current one)
                                        let recent = db.get_recent_tracks(50);

                                        // Filter out current track
                                        let candidates: Vec<_> = recent
                                            .iter()
                                            .filter(|r| r.track_id != current_id)
                                            .collect();

                                        // If we have candidates in history, pick random
                                        if !candidates.is_empty() {
                                            use rand::Rng;
                                            let mut rng = rand::rng();
                                            let random_idx = rng.random_range(0..candidates.len());
                                            let next_record = candidates[random_idx];

                                            info!(
                                                "Randomly selected track from history: {} (ID: {})",
                                                next_record.title, next_record.track_id
                                            );

                                            // Fetch full track data and play (like History screen does)
                                            self.fetch_and_play_track(next_record.track_id);
                                            return; // Don't stop playback
                                        } else {
                                            // Not enough history, try suggestions instead
                                            info!("Not enough history (< 2 tracks), falling back to suggestions");

                                            if !self.content.suggestions_tracks.is_empty() {
                                                // Play first suggestion
                                                let next_track =
                                                    self.content.suggestions_tracks[0].clone();
                                                info!(
                                                    "Playing first suggestion: {}",
                                                    next_track.title
                                                );

                                                self.audio
                                                    .playback_queue
                                                    .load_tracks(vec![next_track]);
                                                if let Some(track) =
                                                    self.audio.playback_queue.current_track()
                                                {
                                                    self.play_track(track.id);
                                                }
                                                return; // Don't stop playback
                                            } else {
                                                info!(
                                                    "No suggestions available, stopping playback"
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("[PlayNext] Failed to access history: {}", e);
                                    }
                                }
                            }
                        }

                        // Default: stop playback
                        info!("End of playlist, stopping playback");
                        self.audio.is_playing = false;
                        self.audio.audio_controller.stop();
                        self.ui.last_playback_error = None;
                        // Reset progress to allow new track selection
                        self.audio.track_start_time = None;
                    }
                }
            }
        }
    }

    /// Check for search results from background tasks
    pub fn check_search_results(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.tasks.search_rx {
            if let Ok(results) = rx.try_recv() {
                self.content.search_loading = false;

                // Replace old page with new page
                self.content.search_results_tracks = results.tracks;
                self.content.search_results_playlists = results.playlists;

                self.content.search_next_href = results.next_href.clone();
                self.content.search_has_more = results.next_href.is_some();

                // Consumed the message, receiver is done
                self.tasks.search_rx = None;

                ctx.request_repaint();
            }
        }
    }

    /// Check for playlist load completion from background tasks
    pub fn check_playlist_load(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.tasks.playlist_rx {
            if let Ok(playlist) = rx.try_recv() {
                log::info!(
                    "[Playlist] Background load complete: {} total tracks",
                    playlist.tracks.len()
                );

                self.content.selected_playlist_id = Some(playlist.id);

                let streamable_tracks: Vec<_> = playlist
                    .tracks
                    .into_iter()
                    .filter(|t| t.streamable.unwrap_or(false) && t.stream_url.is_some())
                    .collect();

                if !streamable_tracks.is_empty() {
                    // When loading from Playlists screen, replace the queue (don't merge)
                    log::info!(
                        "[Playlist] Loading {} tracks into queue",
                        streamable_tracks.len()
                    );

                    // Load playlist into queue (this replaces existing queue)
                    self.audio.playback_queue.load_tracks(streamable_tracks);

                    // Start playing first track
                    if let Some(track) = self.audio.playback_queue.current_track() {
                        log::info!("[Playlist] Playing first track: {}", track.title);
                        self.play_track(track.id);
                    }

                    ctx.request_repaint();
                }

                self.tasks.playlist_rx = None;
            }
        }
    }

    /// Check for home screen data updates from background tasks
    pub fn check_home_updates(&mut self) {
        // Check recently played
        if let Some(rx) = &self.tasks.home_recently_played_rx {
            if let Ok(tracks) = rx.try_recv() {
                let track_count = tracks.len();
                log::info!("[Home] Received {} recently played tracks", track_count);
                self.content.home_content.recently_played = tracks.clone();
                self.content.home_content.initial_fetch_done = true;
                self.content.home_loading = false;
                self.tasks.home_recently_played_rx = None;

                // Only fetch recommendations if we have history to base them on
                if !tracks.is_empty() && !self.content.home_recommendations_loading {
                    // Fetch 6 recommendations based on recently played
                    self.fetch_recommendations(tracks, 6);
                }
            }
        }

        // Check recommendations
        if let Some(rx) = &self.tasks.home_recommendations_rx {
            if let Ok(mut tracks) = rx.try_recv() {
                log::info!("[Home] Received {} recommended tracks", tracks.len());

                // If we have less than 6, fill with history tracks
                if tracks.len() < 6 {
                    let needed = 6 - tracks.len();
                    log::info!("[Home] Filling {} empty slots with history tracks", needed);

                    // Get history tracks that aren't already in recommendations
                    let rec_ids: std::collections::HashSet<u64> =
                        tracks.iter().map(|t| t.id).collect();
                    let history_tracks: Vec<_> = self
                        .content
                        .home_content
                        .recently_played
                        .iter()
                        .filter(|t| !rec_ids.contains(&t.id))
                        .take(needed)
                        .cloned()
                        .collect();

                    tracks.extend(history_tracks);
                }

                // Store recommendations (max 6)
                self.content.home_content.recommendations = tracks
                    .into_iter()
                    .take(HOME_RECOMMENDATIONS_LIMIT)
                    .collect();
                self.content.home_recommendations_loading = false;
                self.tasks.home_recommendations_rx = None;
            }
        }

        // Check suggestions
        if let Some(rx) = &self.tasks.suggestions_rx {
            if let Ok(mut tracks) = rx.try_recv() {
                log::info!("[Suggestions] Received {} suggestion tracks", tracks.len());

                // If we have less than 12, fill with history tracks
                if tracks.len() < 12 {
                    let needed = 12 - tracks.len();
                    log::info!(
                        "[Suggestions] Filling {} empty slots with history tracks",
                        needed
                    );

                    // Get history tracks that aren't already in suggestions
                    let sug_ids: std::collections::HashSet<u64> =
                        tracks.iter().map(|t| t.id).collect();
                    let history_records =
                        self.content.playback_history.get_recent_tracks(needed + 10);
                    let history_tracks: Vec<_> = history_records
                        .iter()
                        .filter(|r| !sug_ids.contains(&r.track_id))
                        .take(needed)
                        .map(|record| crate::app::playlists::Track {
                            id: record.track_id,
                            title: record.title.clone(),
                            user: crate::app::playlists::User {
                                id: 0,
                                username: record.artist.clone(),
                                avatar_url: None,
                            },
                            duration: record.duration,
                            full_duration: None, // Not stored in history DB
                            genre: record.genre.clone(),
                            artwork_url: None,
                            permalink_url: None,
                            stream_url: None,
                            streamable: Some(true),
                            playback_count: None,
                            access: None,
                            policy: None,
                        })
                        .collect();

                    tracks.extend(history_tracks);
                }

                // Store all suggestions for pagination
                self.content.suggestions_tracks = tracks;
                self.content.suggestions_loading = false;
                self.tasks.suggestions_rx = None;
                self.content.suggestions_initial_fetch_done = true;
            }
        }
    }

    /// Fetch home screen data (recently played from local database)
    pub fn fetch_home_data(&mut self) {
        if self.content.home_loading {
            return; // Already loading
        }

        log::info!("[Home] Fetching recently played tracks from local database (ordered by played_at DESC)...");
        self.content.home_loading = true;

        let (tx, rx) = channel();
        self.tasks.home_recently_played_rx = Some(rx);

        // Fetch directly from database - no queue needed
        let token = self
            .auth
            .oauth_manager
            .as_ref()
            .and_then(crate::utils::token_helper::get_valid_token_sync)
            .map(|t| t.access_token.clone())
            .unwrap_or_default();
        crate::app::home::fetch_recently_played_async(token, tx);
    }

    /// Refresh recently played section immediately (after new track starts)
    fn refresh_home_recently_played(&mut self) {
        log::info!("[Home] Refreshing recently played and recommendations after track change...");

        // First, get the current track from queue to use for recommendations
        let current_track = self.audio.playback_queue.current_track().cloned();

        if let Some(track) = current_track {
            // Immediately fetch recommendations based on newly playing track
            if let Some(oauth) = &self.auth.oauth_manager {
                if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                    if !self.content.home_recommendations_loading {
                        log::info!("[Home] Fetching recommendations for newly playing track...");
                        self.content.home_recommendations_loading = true;

                        let (rec_tx, rec_rx) = channel();
                        self.tasks.home_recommendations_rx = Some(rec_rx);

                        // Fetch recommendations immediately
                        crate::app::home::fetch_recommendations_async(
                            token_data.access_token,
                            vec![track],
                            rec_tx,
                            5,
                        );
                    }
                }
            }
        }

        // Then refresh recently played list from database (ordered by played_at DESC)
        let (tx, rx) = channel();
        self.tasks.home_recently_played_rx = Some(rx);

        // Fetch directly from database - no queue needed
        let token = self
            .auth
            .oauth_manager
            .as_ref()
            .and_then(crate::utils::token_helper::get_valid_token_sync)
            .map(|t| t.access_token.clone())
            .unwrap_or_default();
        crate::app::home::fetch_recently_played_async(token, tx);
    }

    /// Fetch recommendations based on recently played tracks
    fn fetch_recommendations(
        &mut self,
        recently_played: Vec<crate::app::playlists::Track>,
        limit: usize,
    ) {
        if self.content.home_recommendations_loading {
            return;
        }

        if let Some(oauth) = &self.auth.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Home] Fetching {} recommendations...", limit);
                self.content.home_recommendations_loading = true;

                let (tx, rx) = channel();
                self.tasks.home_recommendations_rx = Some(rx);

                crate::app::home::fetch_recommendations_async(
                    token_data.access_token,
                    recently_played,
                    tx,
                    limit,
                );
            }
        }
    }

    /// Fetch all suggestions for the Suggestions screen (up to 100 tracks)
    pub fn fetch_all_suggestions(&mut self) {
        if self.content.suggestions_loading {
            return;
        }

        if let Some(oauth) = &self.auth.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Suggestions] Fetching suggestions from multiple sources...");
                self.content.suggestions_loading = true;

                let token = token_data.access_token.clone();
                let (tx, rx) = channel();
                self.tasks.suggestions_rx = Some(rx);

                // Get history tracks for recommendations API
                let recent_tracks = self.content.playback_history.get_recent_tracks(50);
                let history_tracks_for_api: Vec<crate::app::playlists::Track> = recent_tracks
                    .iter()
                    .map(|record| {
                        crate::app::playlists::Track {
                            id: record.track_id,
                            title: record.title.clone(),
                            user: crate::app::playlists::User {
                                id: 0,
                                username: record.artist.clone(),
                                avatar_url: None,
                            },
                            duration: record.duration,
                            full_duration: None, // Not stored in history DB
                            genre: record.genre.clone(),
                            artwork_url: None,
                            permalink_url: None,
                            stream_url: None,
                            streamable: Some(true),
                            playback_count: None,
                            access: None,
                            policy: None,
                        }
                    })
                    .collect();

                // Clone current likes for merging
                let likes_tracks = self.content.likes_tracks.clone();
                let user_tracks = self.content.user_tracks.clone();

                // Get history records upfront (can't clone PlaybackHistoryDB)
                let _history_records = self.content.playback_history.get_recent_tracks(50);

                std::thread::spawn(move || {
                    let rt = match crate::utils::error_handling::create_runtime() {
                        Ok(r) => r,
                        Err(e) => {
                            log::error!("[Suggestions] {}", e);
                            return;
                        }
                    };
                    rt.block_on(async {
                        let mut all_suggestions = Vec::new();
                        let mut seen_ids = std::collections::HashSet::new();

                        // Source 1: Recommended API (primary - fetch related tracks based on recent history)
                        if !history_tracks_for_api.is_empty() {
                            log::info!("[Suggestions] Fetching from Recommended API based on recent history...");
                            if let Some(recent_track) = history_tracks_for_api.first() {
                                let track_urn = format!("soundcloud:tracks:{}", recent_track.id);
                                log::info!("[Suggestions] Using track '{}' for related tracks", recent_track.title);

                                match crate::api::tracks::fetch_related_tracks(&token, &track_urn, 40).await {
                                    Ok(mut recommended) => {
                                        log::info!("[Suggestions] Got {} tracks from Recommended API", recommended.len());
                                        // Add all related tracks
                                        for track in recommended.drain(..) {
                                            if seen_ids.insert(track.id) {
                                                all_suggestions.push(track);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!("[Suggestions] Recommended API failed: {}", e);
                                    }
                                }
                            }
                        }

                        // Source 2: Liked tracks (secondary - add some variety)
                        log::info!("[Suggestions] Adding from {} liked tracks...", likes_tracks.len());
                        for track in likes_tracks.iter().take(SUGGESTIONS_LIKES_LIMIT) {
                            if seen_ids.insert(track.id) {
                                all_suggestions.push(track.clone());
                            }
                        }

                        // Source 3: User uploaded tracks
                        log::info!("[Suggestions] Adding from {} user tracks...", user_tracks.len());
                        for track in user_tracks.iter().take(SUGGESTIONS_USER_TRACKS_LIMIT) {
                            if seen_ids.insert(track.id) {
                                all_suggestions.push(track.clone());
                            }
                        }

                        // History tracks removed - they lack stream_url and cause playback issues

                        log::info!("[Suggestions] Combined {} unique tracks from all sources", all_suggestions.len());
                        let _ = tx.send(all_suggestions);
                    });
                });
            }
        }
    }

    /// Fetch user's liked tracks and uploaded tracks
    pub fn fetch_likes(&mut self) {
        if self.content.likes_loading {
            return;
        }

        if let Some(oauth) = &self.auth.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Likes] Fetching liked tracks and user tracks...");
                self.content.likes_loading = true;

                let token = token_data.access_token.clone();

                // Fetch liked tracks
                let (tracks_tx, tracks_rx) = channel();
                self.tasks.likes_tracks_rx = Some(tracks_rx);

                std::thread::spawn(move || {
                    let rt = match crate::utils::error_handling::create_runtime() {
                        Ok(r) => r,
                        Err(e) => {
                            log::error!("[PlayerApp] {}", e);
                            return;
                        }
                    };
                    rt.block_on(async {
                        match crate::api::likes::fetch_user_liked_tracks(&token).await {
                            Ok(tracks) => {
                                log::info!("[Likes] Fetched {} liked tracks", tracks.len());
                                let _ = tracks_tx.send(tracks);
                            }
                            Err(e) => {
                                log::error!("[Likes] Failed to fetch liked tracks: {}", e);
                            }
                        }
                    });
                });

                // Fetch user's uploaded tracks
                let token_user = token_data.access_token.clone();
                let (user_tx, user_rx) = channel();
                self.tasks.user_tracks_rx = Some(user_rx);

                std::thread::spawn(move || {
                    let rt = match crate::utils::error_handling::create_runtime() {
                        Ok(r) => r,
                        Err(e) => {
                            log::error!("[PlayerApp] {}", e);
                            return;
                        }
                    };
                    rt.block_on(async {
                        match crate::api::likes::fetch_user_tracks(&token_user).await {
                            Ok(tracks) => {
                                log::info!("[Likes] Fetched {} user uploaded tracks", tracks.len());
                                let _ = user_tx.send(tracks);
                            }
                            Err(e) => {
                                log::error!("[Likes] Failed to fetch user tracks: {}", e);
                            }
                        }
                    });
                });
            }
        }
    }

    /// Fetch liked track IDs only (lightweight, for startup)
    /// This populates liked_track_ids HashSet without loading full track data
    pub fn fetch_liked_track_ids_only(&mut self) {
        if let Some(oauth) = &self.auth.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Likes] Fetching liked track IDs for social buttons...");

                let token = token_data.access_token.clone();
                let (tx, rx) = channel();
                self.tasks.likes_tracks_rx = Some(rx);

                std::thread::spawn(move || {
                    let rt = match crate::utils::error_handling::create_runtime() {
                        Ok(r) => r,
                        Err(e) => {
                            log::error!("[PlayerApp] {}", e);
                            return;
                        }
                    };
                    rt.block_on(async {
                        match crate::api::likes::fetch_user_liked_tracks(&token).await {
                            Ok(tracks) => {
                                log::info!("[Likes] Fetched {} liked track IDs", tracks.len());
                                let _ = tx.send(tracks);
                            }
                            Err(e) => {
                                log::error!("[Likes] Failed to fetch liked track IDs: {}", e);
                            }
                        }
                    });
                });
            }
        }
    }

    /// Fetch user's playlists
    pub fn fetch_playlists(&mut self) {
        if self.content.playlists_loading {
            return;
        }

        if let Some(oauth) = &self.auth.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Playlists] Fetching user playlists...");
                self.content.playlists_loading = true;

                let token = token_data.access_token.clone();
                let (playlists_tx, playlists_rx): (_, Receiver<(Vec<_>, Vec<u64>)>) = channel();
                self.tasks.playlists_rx = Some(playlists_rx);

                std::thread::spawn(move || {
                    let rt = match crate::utils::error_handling::create_runtime() {
                        Ok(r) => r,
                        Err(e) => {
                            log::error!("[PlayerApp] {}", e);
                            return;
                        }
                    };
                    rt.block_on(async {
                        match crate::api::likes::fetch_user_playlists(&token).await {
                            Ok((playlists, created_ids)) => {
                                log::info!(
                                    "[Playlists] Fetched {} playlists ({} created)",
                                    playlists.len(),
                                    created_ids.len()
                                );
                                let _ = playlists_tx.send((playlists, created_ids));
                            }
                            Err(e) => {
                                log::error!("[Playlists] Failed to fetch playlists: {}", e);
                            }
                        }
                    });
                });
            }
        }
    }

    /// Check for likes updates from background tasks
    pub fn check_likes_updates(&mut self) {
        let mut pending = 0;

        // Check liked tracks
        if let Some(rx) = &self.tasks.likes_tracks_rx {
            if let Ok(tracks) = rx.try_recv() {
                log::info!("[Likes] Received {} liked tracks", tracks.len());

                // Update liked track IDs HashSet
                self.content.liked_track_ids.clear();
                for track in &tracks {
                    self.content.liked_track_ids.insert(track.id);
                }
                log::info!(
                    "[Likes] Updated liked_track_ids with {} IDs",
                    self.content.liked_track_ids.len()
                );

                self.content.likes_tracks = tracks;
                self.tasks.likes_tracks_rx = None;
            } else {
                pending += 1;
            }
        }

        // Check user uploaded tracks
        if let Some(rx) = &self.tasks.user_tracks_rx {
            if let Ok(tracks) = rx.try_recv() {
                log::info!("[Likes] Received {} user uploaded tracks", tracks.len());
                self.content.user_tracks = tracks;
                self.tasks.user_tracks_rx = None;
            } else {
                pending += 1;
            }
        }

        // Mark loading as complete when all channels are done
        if pending == 0 {
            self.content.likes_loading = false;
        }
    }

    /// Check for playlists updates from background tasks
    pub fn check_playlists_updates(&mut self) {
        if let Some(rx) = &self.tasks.playlists_rx {
            if let Ok((playlists, created_ids)) = rx.try_recv() {
                log::info!(
                    "[Playlists] Received {} playlists ({} created)",
                    playlists.len(),
                    created_ids.len()
                );

                // Track user-created playlist IDs
                self.content.user_created_playlist_ids.clear();
                for id in created_ids {
                    self.content.user_created_playlist_ids.insert(id);
                }

                // Build liked playlist IDs set (all playlists)
                self.content.liked_playlist_ids.clear();
                for playlist in &playlists {
                    self.content.liked_playlist_ids.insert(playlist.id);
                }

                self.content.playlists = playlists;
                self.tasks.playlists_rx = None;
                self.content.playlists_loading = false;
            }
        }
    }

    /// Check for suggestions updates from background tasks
    pub fn check_suggestions_updates(&mut self) {
        if let Some(rx) = &self.tasks.suggestions_rx {
            if let Ok(tracks) = rx.try_recv() {
                log::info!("[Suggestions] Received {} suggestion tracks", tracks.len());
                self.content.suggestions_tracks = tracks;
                self.tasks.suggestions_rx = None;
                self.content.suggestions_loading = false;
            }
        }
    }

    /// Fetch popular tracks for new users with no activity (fallback)
    /// Check if token has expired and trigger re-authentication if needed
    pub fn check_token_expiry(&mut self) {
        let now = Instant::now();

        // Check every 60 seconds
        if let Some(last_check) = self.auth.last_token_check {
            if now.duration_since(last_check) < self.auth.token_check_interval {
                return; // Not time to check yet
            }
        }

        self.auth.last_token_check = Some(now);

        // Only check if we're on the main screen (logged in)
        if !matches!(self.ui.screen, AppScreen::Main) {
            return;
        }

        // Check and refresh token if needed using helper
        if let Some(oauth) = &self.auth.oauth_manager {
            // Don't do anything if refresh is already in progress
            if self.auth.refresh_in_progress {
                log::debug!("[OAuth] Refresh already in progress, waiting...");
                return;
            }

            // Mark refresh as in progress
            self.auth.refresh_in_progress = true;

            let oauth_clone = oauth.clone();

            // Spawn refresh task in background
            std::thread::spawn(move || {
                let rt = match crate::utils::error_handling::create_runtime() {
                    Ok(r) => r,
                    Err(e) => {
                        log::error!("[PlayerApp] {}", e);
                        return;
                    }
                };
                rt.block_on(async {
                    let _ = crate::utils::token_helper::ensure_fresh_token(&oauth_clone).await;
                });
            });
        }
    }

    /// Fetch track data from API and play it (for database tracks with no stream_url)
    pub fn fetch_and_play_track(&mut self, track_id: u64) {
        if let Some(oauth) = &self.auth.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Home] Fetching full track data for ID: {}", track_id);

                // Capture current session for validation in callback
                let session = self.audio.playback_session;

                let token = token_data.access_token.clone();
                let (tx, rx) = channel();

                std::thread::spawn(move || {
                    let rt = match crate::utils::error_handling::create_runtime() {
                        Ok(r) => r,
                        Err(e) => {
                            log::error!("[PlayerApp] {}", e);
                            return;
                        }
                    };
                    rt.block_on(async {
                        // TIMEOUT WRAPPER: Fail fast if API is unresponsive (10s max)
                        let fetch_result = tokio::time::timeout(
                            Duration::from_secs(10),
                            crate::app::playlists::fetch_track_by_id(&token, track_id)
                        ).await;

                        match fetch_result {
                            Ok(Ok(track)) => {
                                // VALIDATE BEFORE RETURNING
                                if !crate::utils::track_filter::is_track_playable(&track) {
                                    log::warn!("[Fetch] Track {} is not playable (geo-blocked or restricted)", track_id);
                                    let _ = tx.send((session, Ok(vec![]))); // Empty = auto-skip
                                } else {
                                    log::info!("[Fetch] Fetched playable track: {}", track.title);
                                    let _ = tx.send((session, Ok(vec![track])));
                                }
                            }
                            Ok(Err(e)) => {
                                let err_msg = e.to_string();
                                // Check if it's a "not playable" or "restricted" error - treat as warning, not fatal
                                if err_msg.contains("not playable") || err_msg.contains("not available") || err_msg.contains("restricted") {
                                    log::warn!("[Fetch] Skipping unavailable track {}: {}", track_id, e);
                                    let _ = tx.send((session, Ok(vec![]))); // Return empty instead of error - triggers auto-skip
                                } else {
                                    log::error!("[Fetch] Failed to fetch track {}: {}", track_id, e);
                                    let _ = tx.send((session, Err(err_msg)));
                                }
                            }
                            Err(_) => {
                                log::error!("[Fetch] Timeout fetching track {} after 10 seconds", track_id);
                                let _ = tx.send((session, Err("API timeout (10s exceeded)".to_string())));
                            }
                        }
                    });
                });

                // Store receiver for checking in update loop
                self.tasks.track_fetch_rx = Some(rx);
            }
        }
    }

    /// Check for fetched track data and play when ready
    fn check_track_fetch(&mut self) {
        if let Some(rx) = &self.tasks.track_fetch_rx {
            if let Ok((session, result)) = rx.try_recv() {
                // Guard: Only apply fetch if session matches
                if session != self.audio.playback_session {
                    log::debug!(
                        "[Fetch] Ignoring stale track fetch (session {} != {})",
                        session,
                        self.audio.playback_session
                    );
                    self.tasks.track_fetch_rx = None;
                    return;
                }

                match result {
                    Ok(tracks) => {
                        if !tracks.is_empty() {
                            log::info!("[Home] Track(s) fetched, loading into queue");
                            self.audio.playback_queue.load_tracks(tracks.clone());
                            if let Some(first_track) = tracks.first() {
                                self.play_track(first_track.id);
                            }
                        } else {
                            log::warn!("[Home] Track fetch returned empty (likely not playable) - auto-skipping to next track");
                            // Auto-skip to next track to avoid infinite loop
                            self.play_next();
                        }
                    }
                    Err(e) => {
                        log::error!("[Home] Track fetch failed: {}", e);
                        self.ui.last_playback_error = Some(format!("Failed to load track: {}", e));
                    }
                }
                self.tasks.track_fetch_rx = None;
            }
        }
    }

    /// Fetch multiple tracks from API and play as playlist
    pub fn fetch_and_play_playlist(&mut self, track_ids: Vec<u64>) {
        if let Some(oauth) = &self.auth.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Home] Fetching {} tracks from API...", track_ids.len());

                // Capture current session for validation in callback
                let session = self.audio.playback_session;

                let token = token_data.access_token.clone();
                let (tx, rx) = channel();

                std::thread::spawn(move || {
                    let rt = match crate::utils::error_handling::create_runtime() {
                        Ok(r) => r,
                        Err(e) => {
                            log::error!("[PlayerApp] {}", e);
                            return;
                        }
                    };
                    rt.block_on(async {
                        let mut tracks = Vec::new();
                        for track_id in track_ids {
                            match crate::app::playlists::fetch_track_by_id(&token, track_id).await {
                                Ok(track) => tracks.push(track),
                                Err(e) => log::warn!("[Home] Skipping track {}: {}", track_id, e),
                            }
                        }
                        if !tracks.is_empty() {
                            let _ = tx.send((session, Ok(tracks)));
                        } else {
                            let _ = tx.send((session, Err("No playable tracks found".to_string())));
                        }
                    });
                });

                self.tasks.track_fetch_rx = Some(rx);
            }
        }
    }
}

impl eframe::App for MusicPlayerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Adaptive frame rate based on renderer type and activity
        // GPU: 120 FPS for smooth shader animations
        // CPU Active: 30 FPS when loading/toasts (smooth UI feedback)
        // CPU Idle: 20 FPS when nothing happening (maximum power savings)
        let repaint_interval =
            if self.content.app_state.get_renderer_type() == crate::app_state::RendererType::Gpu {
                Duration::from_micros(REPAINT_INTERVAL_GPU_MICROS)
            } else {
                // Check if there's any activity requiring smoother updates
                let is_active = self.content.search_loading
                    || self.content.home_loading
                    || self.ui.artwork_loading
                    || !self.ui.toast_manager.toasts.is_empty();

                if is_active {
                    Duration::from_micros(REPAINT_INTERVAL_CPU_ACTIVE) // 30 FPS
                } else {
                    Duration::from_micros(REPAINT_INTERVAL_CPU_IDLE) // 20 FPS
                }
            };
        ctx.request_repaint_after(repaint_interval);

        // Check for shader hot-reload (delegated to ShaderManager)
        // Only in GPU mode - no shaders loaded in CPU mode
        if self.content.app_state.get_renderer_type() == crate::app_state::RendererType::Gpu
            && (matches!(self.ui.selected_tab, MainTab::NowPlaying)
                || matches!(self.ui.screen, AppScreen::Splash))
        {
            self.ui.shader_manager.check_hot_reload();
        }

        // Handle close request - cleanup and exit immediately
        if ctx.input(|i| i.viewport().close_requested()) && !self.ui.is_shutting_down {
            self.ui.is_shutting_down = true;
            self.cleanup_and_exit(ctx, frame);
        }
        // Handle OAuth authentication flow and token validation
        if matches!(self.ui.screen, AppScreen::Splash) {
            // Check for existing valid token (only once per session)
            if !self.auth.token_check_done {
                self.auth.token_check_done = true;

                if let Some(oauth_manager) = &self.auth.oauth_manager {
                    // Use helper to check and refresh token if needed
                    if crate::utils::token_helper::ensure_fresh_token_sync(oauth_manager) {
                        log::info!("[OAuth] Valid token found on startup!");
                        // Don't transition yet - let the timer check below handle it
                    } else {
                        log::info!("[OAuth] No valid token - user needs to login");
                    }
                }
            }

            // Check if we have a valid token AND minimum splash time has elapsed
            let has_valid_token = if let Some(oauth_manager) = &self.auth.oauth_manager {
                oauth_manager.get_token().is_some()
            } else {
                false
            };

            if has_valid_token {
                // Check if minimum splash duration has elapsed
                let can_transition = if let Some(start_time) = self.ui.splash_start_time {
                    let elapsed = start_time.elapsed();
                    let min_duration = self.ui.splash_min_duration;

                    if elapsed < min_duration {
                        // Not enough time has passed, request repaint to check again soon
                        ctx.request_repaint_after(Duration::from_millis(
                            SPLASH_CHECK_INTERVAL_MILLIS,
                        ));
                        false
                    } else {
                        debug!(
                            "[Splash] Timer check - elapsed: {:?}, minimum: {:?}",
                            elapsed, min_duration
                        );
                        true
                    }
                } else {
                    true // If no timer, allow immediate transition
                };

                if can_transition {
                    log::info!(
                        "[Splash] Minimum display time elapsed, transitioning to main screen"
                    );
                    self.auth.is_authenticating = false;

                    // Shader manager retains shaders - they're reused efficiently

                    self.ui.screen = AppScreen::Main;
                    // Fetch user info (avatar, username) after login
                    self.fetch_user_info();
                    // Fetch liked track IDs for social buttons
                    self.fetch_liked_track_ids_only();
                }
            }
        }

        // Apply dark theme styling with refined color palette
        let mut visuals = egui::Visuals::dark();
        visuals.dark_mode = true;
        visuals.override_text_color = Some(crate::ui_components::colors::TEXT_PRIMARY);
        visuals.panel_fill = crate::ui_components::colors::BG_CARD;
        visuals.window_fill = crate::ui_components::colors::BG_CARD;
        visuals.extreme_bg_color = crate::ui_components::colors::BG_MAIN;

        ctx.set_visuals(visuals);

        // Handle keyboard shortcuts
        if matches!(self.ui.screen, AppScreen::Main) {
            self.handle_keyboard_shortcuts(ctx);
        }

        // Disable text selection globally
        ctx.style_mut(|style| {
            style.interaction.selectable_labels = false;
        });

        // Check for artwork updates
        self.check_artwork(ctx);

        // Check for user avatar updates
        self.check_user_avatar(ctx);

        // Check for playlist chunk updates
        self.check_playlist_chunks();

        // Background updates only for the active tab to avoid hidden work
        match self.ui.selected_tab {
            MainTab::Home => self.check_home_updates(),
            MainTab::Search => self.check_search_results(ctx),
            MainTab::Playlists => {
                self.check_playlist_load(ctx);
                self.check_playlists_updates();
            }
            MainTab::Suggestions => self.check_suggestions_updates(),
            MainTab::Likes => self.check_likes_updates(),
            MainTab::History => { /* history updates handled on demand */ }
            MainTab::NowPlaying => { /* visuals only; playback runs regardless */ }
        }

        // Check for fetched track data (from database tracks)
        self.check_track_fetch();

        // Check if token has expired (every 60 seconds)
        self.check_token_expiry();

        // Check prefetch progress and completion
        self.check_prefetch_trigger();
        self.check_prefetch_updates();

        // Check if track finished for auto-play
        if matches!(self.ui.screen, AppScreen::Main) {
            self.check_track_finished();
        }

        match self.ui.screen {
            AppScreen::Splash => {
                crate::screens::render_splash_screen(self, ctx);
            }
            AppScreen::Main => {
                // AUDIO REACTIVITY: Use real FFT analysis (lock-free!)
                // Only in GPU mode - FFT is disabled in CPU mode
                if self.content.app_state.get_renderer_type() == crate::app_state::RendererType::Gpu
                    && self.audio.is_playing
                {
                    // Read bass energy for overall amplitude (pulsing effect)
                    self.ui.audio_amplitude =
                        crate::utils::error_handling::load_f32_atomic(&self.audio.bass_energy);
                } else {
                    self.ui.audio_amplitude = 0.0;
                }

                crate::ui_components::layout::render_with_layout(self, ctx);
            }
        }

        // Render toasts on top of everything
        egui::Area::new(egui::Id::new("toast_area"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                self.ui.toast_manager.render(ui);
            });

        // Repaint is already handled by request_repaint_after() throttling earlier
        // (30 FPS in CPU mode when playing, 20 FPS idle)
    }
}
