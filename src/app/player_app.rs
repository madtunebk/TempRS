use eframe::egui;
use crate::utils::oauth::OAuthManager;
use crate::utils::audio_controller::AudioController;
use crate::utils::playback_history::PlaybackHistoryDB;
use log::{info, warn, error, debug};
use crate::app::queue::PlaybackQueue;
use crate::app::playlists::Track as APITrack;
use crate::app::home::HomeContent;
use crate::app_state::{AppState, RepeatMode};
use std::sync::mpsc::{Receiver, channel};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub enum AppScreen {
    Splash,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainTab {
    Home,
    NowPlaying,
    Search,
    History,
    Suggestions,
    Likes,
    Playlists,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchType {
    Tracks,
    Playlists,
}

#[allow(dead_code)]
pub struct MusicPlayerApp {
    pub screen: AppScreen,
    pub selected_tab: MainTab,
    pub logo_texture: Option<egui::TextureHandle>,
    
    // Shared app state
    pub app_state: AppState,
    
    // Toast notification system
    pub toast_manager: crate::ui_components::toast::ToastManager,
    
    // Audio playback
    pub audio_controller: AudioController,
    pub playback_queue: PlaybackQueue,
    
    // Current track info
    pub current_track_id: Option<u64>,
    pub last_track_id: Option<u64>, // Track last playing track for auto-scroll
    pub current_title: String,
    pub current_artist: String,
    pub current_genre: Option<String>,
    pub current_duration_ms: u64,
    pub current_stream_url: Option<String>,
    pub current_permalink_url: Option<String>,
    pub track_start_time: Option<Instant>,
    

    
    // Artwork
    pub artwork_texture: Option<egui::TextureHandle>,
    pub artwork_loading: bool,
    pub artwork_rx: Option<Receiver<egui::ColorImage>>,
    pub artwork_dominant_color: egui::Color32,
    pub artwork_edge_colors: [egui::Color32; 4], // Ambilight effect: [top, right, bottom, left]
    pub thumb_cache: HashMap<String, egui::TextureHandle>,
    pub thumb_pending: HashMap<String, bool>,
    pub no_artwork_texture: Option<egui::TextureHandle>,
    
    // Ambient glow
    pub glow_intensity: f32,
    pub glow_smooth_intensity: f32,
    pub last_frame_time: Option<Instant>,
    pub audio_amplitude: f32,  // Real-time audio level (0.0-1.0) for reactive visuals
    pub last_playback_error: Option<String>, // Track last playback error for UI display
    
    // Real-time FFT audio analysis
    pub bass_energy: std::sync::Arc<std::sync::Mutex<f32>>,
    pub mid_energy: std::sync::Arc<std::sync::Mutex<f32>>,
    pub high_energy: std::sync::Arc<std::sync::Mutex<f32>>,
    
    // Playback controls
    pub is_playing: bool,
    pub shuffle_mode: bool,
    pub repeat_mode: RepeatMode,
    pub volume: f32,
    pub muted: bool,
    pub volume_before_mute: f32,
    pub show_volume_popup: bool,
    
    // Shutdown handling
    pub show_exit_confirmation: bool,
    pub is_shutting_down: bool,
    
    // Seeking
    pub is_seeking: bool,
    pub seek_target_pos: Option<Duration>,
    
    // OAuth
    pub oauth_manager: Option<OAuthManager>,
    pub is_authenticating: bool,
    pub login_message_shown: bool,
    pub refresh_attempted: bool,
    pub token_check_done: bool, // Cache flag to prevent DB spam on every frame
    pub refresh_in_progress: bool, // Prevent token clear while refresh is active
    
    // User info
    pub user_avatar_url: Option<String>,
    pub user_avatar_texture: Option<egui::TextureHandle>,
    pub user_avatar_rx: Option<Receiver<egui::ColorImage>>,
    pub user_username: Option<String>,
    pub show_user_menu: bool,
    
    // Token validation
    pub last_token_check: Option<Instant>,
    pub token_check_interval: Duration,
    
    // Search state
    pub search_query: String,
    pub search_type: SearchType,
    pub search_expanded: bool,
    pub search_results_tracks: Vec<APITrack>,
    pub search_results_playlists: Vec<crate::app::playlists::Playlist>,
    pub search_loading: bool,
    pub search_next_href: Option<String>,
    pub search_has_more: bool,
    pub search_rx: Option<Receiver<SearchResults>>,
    pub search_page: usize,
    pub search_page_size: usize,
    pub playlist_rx: Option<Receiver<crate::app::playlists::Playlist>>,
    pub playlist_chunk_rx: Option<Receiver<Vec<crate::app::playlists::Track>>>,
    
    // Playlist view
    pub selected_playlist_id: Option<u64>,
    pub playlist_loading_id: Option<u64>,
    
    // Home screen content
    pub home_content: HomeContent,
    pub home_recently_played_rx: Option<Receiver<Vec<crate::app::playlists::Track>>>,
    pub home_recommendations_rx: Option<Receiver<Vec<crate::app::playlists::Track>>>,
    pub home_loading: bool,
    pub home_recommendations_loading: bool,
    pub track_fetch_rx: Option<Receiver<Result<Vec<crate::app::playlists::Track>, String>>>,

    // Suggestions screen (paginated recommendations)
    pub suggestions_tracks: Vec<crate::app::playlists::Track>,
    pub suggestions_page: usize,
    pub suggestions_page_size: usize,
    pub suggestions_loading: bool,
    pub suggestions_rx: Option<Receiver<Vec<crate::app::playlists::Track>>>,
    pub suggestions_initial_fetch_done: bool,

    // Likes screen (liked tracks + user uploaded tracks)
    pub likes_tracks: Vec<crate::app::playlists::Track>,
    pub user_tracks: Vec<crate::app::playlists::Track>,
    pub likes_page: usize,
    pub likes_page_size: usize,
    pub likes_loading: bool,
    pub likes_tracks_rx: Option<Receiver<Vec<crate::app::playlists::Track>>>,
    pub user_tracks_rx: Option<Receiver<Vec<crate::app::playlists::Track>>>,
    pub likes_initial_fetch_done: bool,
    pub liked_track_ids: std::collections::HashSet<u64>,  // Track IDs that are liked


    // Playlists screen (user's playlists)
    pub playlists: Vec<crate::app::playlists::Playlist>,
    pub liked_playlist_ids: std::collections::HashSet<u64>,  // Playlist IDs that are liked
    pub user_created_playlist_ids: std::collections::HashSet<u64>,  // Playlist IDs created by user
    pub playlists_page: usize,
    pub playlists_page_size: usize,
    pub playlists_loading: bool,
    pub playlists_rx: Option<Receiver<(Vec<crate::app::playlists::Playlist>, Vec<u64>)>>,
    pub playlists_initial_fetch_done: bool,

    // Playback history database
    pub playback_history: PlaybackHistoryDB,
    
    // History view pagination
    pub history_page: usize,           // Current page (0-indexed)
    pub history_page_size: usize,      // Tracks per page
    pub history_total_tracks: usize,   // Total tracks in DB
    pub history_search_filter: String, // Search/filter text
    pub history_sort_order: crate::screens::history::HistorySortOrder, // Sort order

    // Shader pipeline for background effects (lazy loaded per screen)
    pub splash_shader: Option<std::sync::Arc<crate::utils::shader::ShaderPipeline>>,
    pub track_metadata_shader: Option<std::sync::Arc<crate::utils::shader::ShaderPipeline>>,
    
    // WGPU resources for lazy shader loading
    pub wgpu_device: Option<std::sync::Arc<egui_wgpu::wgpu::Device>>,
    pub wgpu_format: Option<egui_wgpu::wgpu::TextureFormat>,
    
    // Splash screen timer
    pub splash_start_time: Option<Instant>,
    pub splash_min_duration: Duration,
    
    // UI state
    pub queue_collapsed: bool,  // Toggle queue sidebar visibility

}

pub struct SearchResults {
    pub tracks: Vec<APITrack>,
    pub playlists: Vec<crate::app::playlists::Playlist>,
    pub next_href: Option<String>,
}

impl Default for MusicPlayerApp {
    fn default() -> Self {
        // Cache cleanup disabled - was causing app freezing on startup
        // TODO: Re-enable with better async implementation later
        // std::thread::spawn(|| {
        //     if let Ok((age_deleted, size_deleted)) = crate::utils::cache::cleanup_cache_with_limits(30, 100) {
        //         if age_deleted > 0 || size_deleted > 0 {
        //             println!("[Cache] Startup cleanup: {} old entries (>30 days), {} over-limit entries", age_deleted, size_deleted);
        //         }
        //     }
        // });
        
        // Initialize OAuth manager with credentials from main.rs
        let oauth_manager = {
            use crate::utils::oauth::OAuthConfig;
            
            let client_id = crate::SOUNDCLOUD_CLIENT_ID.to_string();
            let client_secret = crate::SOUNDCLOUD_CLIENT_SECRET.to_string();
            let redirect_uri = "http://localhost:3000/callback".to_string();
            let config = OAuthConfig::new(client_id, client_secret, redirect_uri);
            
            OAuthManager::new(config)
        };
        
        let app_state = AppState::new();
        let volume = app_state.get_volume();
        let muted = app_state.is_muted();
        let shuffle_mode = app_state.get_shuffle_mode();
        let repeat_mode = app_state.get_repeat_mode();
        let volume_before_mute = if muted { volume } else { 0.7 };

        Self {
            screen: AppScreen::Splash,
            selected_tab: MainTab::Home,
            logo_texture: None,
            
            // Start splash timer immediately
            splash_start_time: Some(Instant::now()),
            splash_min_duration: Duration::from_secs(0),
            
            // Shared state
            app_state,
            
            // Toast notifications
            toast_manager: crate::ui_components::toast::ToastManager::new(),
            
            // Real-time FFT analysis
            bass_energy: std::sync::Arc::new(std::sync::Mutex::new(0.0)),
            mid_energy: std::sync::Arc::new(std::sync::Mutex::new(0.0)),
            high_energy: std::sync::Arc::new(std::sync::Mutex::new(0.0)),
            
            // Audio - AudioController will be reassigned after struct is built
            audio_controller: AudioController::new(
                std::sync::Arc::new(std::sync::Mutex::new(0.0)),
                std::sync::Arc::new(std::sync::Mutex::new(0.0)),
                std::sync::Arc::new(std::sync::Mutex::new(0.0)),
            ),
            playback_queue: PlaybackQueue::new(),
            
            // Current track
            current_track_id: None,
            last_track_id: None,
            current_title: String::new(),
            current_artist: String::new(),
            current_genre: None,
            current_duration_ms: 0,
            current_stream_url: None,
            current_permalink_url: None,
            track_start_time: None,
            
            // Artwork
            artwork_texture: None,
            artwork_loading: false,
            artwork_rx: None,
            artwork_dominant_color: egui::Color32::from_rgb(255, 85, 0),
            artwork_edge_colors: [
                egui::Color32::from_rgb(255, 85, 0),
                egui::Color32::from_rgb(255, 85, 0),
                egui::Color32::from_rgb(255, 85, 0),
                egui::Color32::from_rgb(255, 85, 0),
            ],
            thumb_cache: HashMap::new(),
            thumb_pending: HashMap::new(),
            no_artwork_texture: None,
            
            // Ambient glow
            glow_intensity: 0.0,
            glow_smooth_intensity: 0.0,
            last_frame_time: None,
            audio_amplitude: 0.0,
            last_playback_error: None,
            
            // Playback
            is_playing: false,
            shuffle_mode,
            repeat_mode,
            volume,
            muted,
            volume_before_mute,
            show_volume_popup: false,
            
            // Shutdown
            show_exit_confirmation: false,
            is_shutting_down: false,
            
            // Seeking
            is_seeking: false,
            seek_target_pos: None,
            
            // OAuth
            oauth_manager: Some(oauth_manager),
            is_authenticating: false,
            login_message_shown: false,
            refresh_attempted: false,
            token_check_done: false,
            refresh_in_progress: false,
            
            // User info
            user_avatar_url: None,
            user_avatar_texture: None,
            user_avatar_rx: None,
            user_username: None,
            show_user_menu: false,
            
            // Token validation
            last_token_check: None,
            token_check_interval: Duration::from_secs(60), // Check every 60 seconds
            
            // Search
            search_query: String::new(),
            search_type: SearchType::Tracks,
            search_expanded: false,
            search_results_tracks: Vec::new(),
            search_results_playlists: Vec::new(),
            search_loading: false,
            search_next_href: None,
            search_has_more: false,
            search_rx: None,
            search_page: 0,
            search_page_size: 12, // 2-3 rows x 5 cards
            playlist_rx: None,
            playlist_chunk_rx: None,
            selected_playlist_id: None,
            playlist_loading_id: None,
            
            // Home content
            home_content: HomeContent::new(),
            home_recently_played_rx: None,
            home_recommendations_rx: None,
            home_loading: false,
            home_recommendations_loading: false,
            track_fetch_rx: None,

            // Suggestions screen
            suggestions_tracks: Vec::new(),
            suggestions_page: 0,
            suggestions_page_size: 12,  // 2-3 rows x 5 cards
            suggestions_loading: false,
            suggestions_rx: None,
            suggestions_initial_fetch_done: false,

            // Likes screen
            likes_tracks: Vec::new(),
            user_tracks: Vec::new(),
            likes_page: 0,
            likes_page_size: 12,
            likes_loading: false,
            likes_tracks_rx: None,
            user_tracks_rx: None,
            likes_initial_fetch_done: false,
            liked_track_ids: std::collections::HashSet::new(),


            // Playlists screen
            playlists: Vec::new(),
            liked_playlist_ids: std::collections::HashSet::new(),
            user_created_playlist_ids: std::collections::HashSet::new(),
            playlists_page: 0,
            playlists_page_size: 12,
            playlists_loading: false,
            playlists_rx: None,
            playlists_initial_fetch_done: false,

            // Playback history
            playback_history: PlaybackHistoryDB::default(),
            history_page: 0,
            history_page_size: 12,  // 2-3 rows x 5 cards
            history_total_tracks: 0,
            history_search_filter: String::new(),
            history_sort_order: crate::screens::history::HistorySortOrder::RecentFirst,

            // Shader pipeline for background effects (lazy loaded)
            splash_shader: None,
            track_metadata_shader: None,
            
            // WGPU resources for lazy loading
            wgpu_device: None,
            wgpu_format: None,
            
            // UI state
            queue_collapsed: false,  // Queue visible by default
        }
    }
}

impl MusicPlayerApp {
    /// Create a new MusicPlayerApp with shader initialized from eframe CreationContext
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        
        // Recreate AudioController with proper FFT handles
        app.audio_controller = AudioController::new(
            Arc::clone(&app.bass_energy),
            Arc::clone(&app.mid_energy),
            Arc::clone(&app.high_energy),
        );
        
        // Initialize shaders if WGPU is available
        if let Some(render_state) = cc.wgpu_render_state.as_ref() {
            let device = &render_state.device;
            let format = render_state.target_format;
            
            // Splash screen shader (Nebula Drift)
            let splash_wgsl = include_str!("../shaders/splash_bg.wgsl");
            let splash_pipeline = crate::utils::shader::ShaderPipeline::new(device, format, splash_wgsl);
            app.splash_shader = Some(std::sync::Arc::new(splash_pipeline));
            log::info!("[Shader] Loaded splash shader (Nebula Drift)");
            
            // Track metadata shader (loaded but only used if component is shown)
            let metadata_wgsl = include_str!("../shaders/track_metadata_bg.wgsl");
            let metadata_pipeline = crate::utils::shader::ShaderPipeline::new(device, format, metadata_wgsl);
            app.track_metadata_shader = Some(std::sync::Arc::new(metadata_pipeline));
            log::info!("[Shader] Loaded track metadata shader");
            
            log::info!("[Shader] All shaders initialized. Format: {:?}", format);
        } else {
            log::warn!("[Shader] WGPU render state not available");
        }
        
        app
    }
    
    /// Save playback configuration to app state
    pub fn save_playback_config(&self) {
        self.app_state.set_volume(self.volume);
        self.app_state.set_muted(self.muted);
        self.app_state.set_shuffle_mode(self.shuffle_mode);
        self.app_state.set_repeat_mode(self.repeat_mode);
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
                self.artwork_rx = Some(rx);
                let _ = tx.send(img);
                self.artwork_loading = false;
                return;
            }
        }
        
        self.artwork_loading = true;
        self.artwork_texture = None;
        
        let (_cancel_tx, rx) = crate::utils::artwork::fetch_artwork(track_id, artwork_url.to_string());
        self.artwork_rx = Some(rx);
    }
    
    /// Check for received artwork from background thread
    pub fn check_artwork(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.artwork_rx {
            if let Ok(img) = rx.try_recv() {
                // Extract dominant color for ambient glow
                self.artwork_dominant_color = crate::utils::artwork::extract_dominant_color(&img);
                
                // Extract edge colors for Ambilight effect
                self.artwork_edge_colors = crate::utils::artwork::extract_edge_colors(&img);
                
                self.artwork_texture = Some(ctx.load_texture(
                    "artwork",
                    img,
                    egui::TextureOptions::LINEAR,
                ));
                self.artwork_loading = false;
                self.artwork_rx = None;
            }
        }
    }

    /// Play a track by ID
    pub fn play_track(&mut self, track_id: u64) {
        info!("[PLAY] play_track({}) called - is_playing={}, current_track_id={:?}", track_id, self.is_playing, self.current_track_id);
        
        // Don't send stop command - the audio controller will replace the old player automatically
        // This prevents interrupting the download of new track
        self.is_playing = false; // Temporarily set to false, will be set to true when playback starts
        
        // Clear previous errors
        self.last_playback_error = None;
        
        // Note: Token validity is checked by periodic check_token_expiry() which runs every 60s
        // and automatically refreshes before expiry. No need to check here.
        
        // Update queue position to the selected track
        self.playback_queue.jump_to_track_id(track_id);
        
        // Get track from queue (which has the current tracks loaded)
        let track = match self.playback_queue.current_track() {
            Some(t) => t.clone(),
            None => {
                let error_msg = format!("Track {} not found in queue", track_id);
                warn!("{}", error_msg);
                self.last_playback_error = Some(error_msg);
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
            let error_msg = format!("Track '{}' is not playable (geo-blocked or preview-only)", track.title);
            log::warn!("{}", error_msg);
            self.last_playback_error = Some(error_msg);
            
            // Auto-skip to next track instead of stopping playback
            log::info!("[PLAY] Auto-skipping to next track...");
            self.play_next();
            return;
        }

        // Clone data we need
        let artwork_url = track.artwork_url.clone();

        // Update current track info
        self.current_track_id = Some(track.id);
        self.current_title = track.title.clone();
        self.current_artist = track.user.username.clone();
        self.current_genre = track.genre.clone();
        self.current_duration_ms = track.duration;
        self.current_stream_url = track.stream_url.clone();
        self.current_permalink_url = track.permalink_url.clone();
        
        // Fetch artwork if available, otherwise clear old artwork
        if let Some(url) = artwork_url {
            self.request_artwork_fetch(track.id, &url);
        } else {
            // No artwork for this track - clear previous artwork
            self.artwork_texture = None;
        }
        
        // Start playback if we have a stream URL
        if let (Some(stream_url), Some(oauth)) = (&self.current_stream_url, &self.oauth_manager) {
            if let Some(token) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("Playing: {} by {}", self.current_title, self.current_artist);
                self.audio_controller.play(stream_url.clone(), token.access_token.clone(), track.id);
                self.is_playing = true;
                log::info!("[PLAY] Playback started - is_playing={}", self.is_playing);
                self.track_start_time = Some(Instant::now());
                
                // Record this track to playback history (only when actually played)
                crate::app::queue::record_track_to_history(&track);
                
                // Refresh Home screen to show newly played track
                self.refresh_home_recently_played();
            } else {
                let error_msg = "Failed to get authentication token";
                error!("{}", error_msg);
                self.last_playback_error = Some(error_msg.to_string());
            }
        } else {
            let error_msg = if self.current_stream_url.is_none() {
                format!("Track '{}' has no stream URL (not streamable)", self.current_title)
            } else {
                "Authentication required".to_string()
            };
            error!("{}", error_msg);
            self.last_playback_error = Some(error_msg);
        }
    }
    
    /// Toggle play/pause
    pub fn toggle_playback(&mut self) {
        log::info!("[TOGGLE] toggle_playback called - is_playing={}, has_track={}", self.is_playing, self.current_track_id.is_some());
        
        // Don't do anything if no track is loaded
        if self.current_track_id.is_none() {
            log::warn!("[TOGGLE] Ignoring toggle - no track loaded");
            return;
        }
        
        if self.is_playing {
            log::info!("[TOGGLE] Pausing playback");
            self.audio_controller.pause();
            self.is_playing = false;
        } else {
            // Check if track was stopped (track_start_time is None) or finished
            if self.track_start_time.is_none() || self.audio_controller.is_finished() {
                // Track was stopped or finished, restart from beginning
                if let Some(track_id) = self.current_track_id {
                    log::info!("[TOGGLE] Track finished, restarting from beginning");
                    // Reset timing for restart
                    self.track_start_time = Some(std::time::Instant::now());
                    self.play_track(track_id);
                }
            } else {
                // Normal resume from pause
                log::info!("[TOGGLE] Resuming playback");
                self.audio_controller.resume();
                self.is_playing = true;
            }
        }
    }
    
    /// Stop playback and reset state (ready to play another track)
    pub fn stop_playback(&mut self) {
        log::info!("[STOP] Stopping playback - clearing track state to hide player controls");
        self.audio_controller.stop();
        self.is_playing = false;
        self.last_playback_error = None;
        // Clear track ID to hide player controls
        self.current_track_id = None;
        // Reset track timing so it restarts from beginning
        self.track_start_time = None;
    }
    
    /// Gracefully cleanup all resources before exit
    fn cleanup_and_exit(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        log::info!("[Shutdown] Starting graceful cleanup...");
        
        // 1. Stop audio playback and cleanup audio threads
        if self.is_playing {
            log::info!("[Shutdown] Stopping audio playback...");
            self.audio_controller.stop();
            self.is_playing = false;
        }
        
        // Explicitly drop audio controller to free resources
        log::info!("[Shutdown] Releasing audio resources...");
        let _ = &mut self.audio_controller;
        
        // 2. Save playback configuration
        log::info!("[Shutdown] Saving playback configuration...");
        self.save_playback_config();
        
        // 3. Clear all pending receivers to prevent thread leaks
        log::info!("[Shutdown] Clearing pending background tasks...");
        self.artwork_rx = None;
        self.user_avatar_rx = None;
        self.search_rx = None;
        self.playlist_rx = None;
        self.playlist_chunk_rx = None;
        self.home_recently_played_rx = None;
        self.home_recommendations_rx = None;
        self.track_fetch_rx = None;
        self.suggestions_rx = None;
        
        // 4. Clear texture caches
        log::info!("[Shutdown] Clearing texture caches...");
        self.thumb_cache.clear();
        self.artwork_texture = None;
        self.user_avatar_texture = None;
        self.no_artwork_texture = None;
        
        // 5. OAuth manager cleanup (tokens are already encrypted in DB)
        log::info!("[Shutdown] Cleaning up OAuth resources...");
        if self.oauth_manager.is_some() {
            // OAuth tokens are persisted in encrypted database, safe to drop
            self.oauth_manager = None;
        }
        
        // 6. Shader cleanup
        if self.splash_shader.is_some() {
            log::info!("[Shutdown] Releasing shader resources...");
            self.splash_shader = None;
        }
        if self.track_metadata_shader.is_some() {
            self.track_metadata_shader = None;
        }
        
        log::info!("[Shutdown] Cleanup complete, closing application...");
        
        // Close the application
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    
    /// Reset to clean state (no track loaded)
    #[allow(dead_code)]
    pub fn reset_player_state(&mut self) {
        info!("[RESET] Resetting player to clean state");
        info!("[RESET] Before: is_playing={}, current_track_id={:?}", self.is_playing, self.current_track_id);
        self.audio_controller.stop();
        self.is_playing = false;
        self.last_playback_error = None; // Clear any error on reset
        self.current_track_id = None;
        self.current_title = String::new();
        self.current_artist = String::new();
        self.current_genre = None;
        self.current_duration_ms = 0;
        self.current_stream_url = None;
        self.track_start_time = None;
        self.artwork_texture = None;
        log::info!("[RESET] After: is_playing={}, current_track_id={:?}", self.is_playing, self.current_track_id);
    }

    /// Play next track in queue
    pub fn play_next(&mut self) {
        let next_track_id = self.playback_queue.next().map(|t| t.id);
        
        if let Some(track_id) = next_track_id {
            self.play_track(track_id);
        } else if self.repeat_mode == RepeatMode::All {
            // Loop back to start
            let first_track_id = self.playback_queue.loop_to_start().map(|t| t.id);
            if let Some(track_id) = first_track_id {
                self.play_track(track_id);
            }
        }
    }

    /// Play previous track in queue
    pub fn play_previous(&mut self) {
        let prev_track_id = self.playback_queue.previous().map(|t| t.id);
        
        if let Some(track_id) = prev_track_id {
            self.play_track(track_id);
        }
    }

    /// Toggle shuffle mode
    pub fn toggle_shuffle(&mut self) {
        self.shuffle_mode = !self.shuffle_mode;
        self.playback_queue.set_shuffle(self.shuffle_mode);
        self.save_playback_config();
        if self.shuffle_mode {
            info!("Shuffle enabled");
        } else {
            info!("Shuffle disabled");
        }
    }

    /// Cycle repeat mode
    pub fn cycle_repeat_mode(&mut self) {
        self.repeat_mode = match self.repeat_mode {
            RepeatMode::None => {
                info!("Repeat All enabled");
                RepeatMode::All
            },
            RepeatMode::All => {
                info!("Repeat One enabled");
                // Disable shuffle when switching to Repeat One
                if self.shuffle_mode {
                    self.shuffle_mode = false;
                    self.playback_queue.set_shuffle(false);
                    info!("Shuffle auto-disabled (incompatible with Repeat One)");
                }
                RepeatMode::One
            },
            RepeatMode::One => {
                info!("Repeat disabled");
                RepeatMode::None
            },
        };
        self.save_playback_config();
    }

    /// Set volume
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.audio_controller.set_volume(self.volume);
        self.save_playback_config();
    }

    /// Toggle mute
    pub fn toggle_mute(&mut self) {
        if self.muted {
            self.volume = self.volume_before_mute;
            self.muted = false;
        } else {
            self.volume_before_mute = self.volume;
            self.volume = 0.0;
            self.muted = true;
        }
        self.audio_controller.set_volume(self.volume);
        self.save_playback_config();
    }

    /// Seek to position
    pub fn seek_to(&mut self, position: Duration) {
        self.audio_controller.seek(position);
        self.is_seeking = true;
        self.seek_target_pos = Some(position);
    }

    /// Get current playback position
    pub fn get_position(&self) -> Duration {
        // Always return actual audio position, UI handles seek preview
        self.audio_controller.get_position()
    }

    /// Get track duration
    pub fn get_duration(&self) -> Duration {
        self.audio_controller
            .get_duration()
            .unwrap_or(Duration::from_millis(self.current_duration_ms))
    }

    /// Check if current track is liked
    pub fn is_current_track_liked(&self) -> bool {
        if let Some(track_id) = self.current_track_id {
            self.liked_track_ids.contains(&track_id)
        } else {
            false
        }
    }

    /// Toggle like status of current track
    pub fn toggle_current_track_like(&mut self) {
        if let Some(track_id) = self.current_track_id {
            self.toggle_like(track_id);
        } else {
            log::warn!("[Like] No track currently playing");
        }
    }
    
    /// Toggle like status for any track by ID
    pub fn toggle_like(&mut self, track_id: u64) {
        let is_liked = self.liked_track_ids.contains(&track_id);
        
        if is_liked {
            // Unlike the track
            log::info!("[Like] Unliking track {}", track_id);
            self.liked_track_ids.remove(&track_id);
            
            // Show optimistic toast
            self.toast_manager.show_info("Removed from Liked tracks");
            
            // Spawn background task to unlike via API
            if let Some(token) = self.app_state.get_token() {
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        match crate::api::likes::unlike_track(&token, track_id).await {
                            Ok(_) => log::info!("[Like] Successfully unliked track {}", track_id),
                            Err(e) => log::error!("[Like] Failed to unlike track {}: {}", track_id, e),
                        }
                    });
                });
            } else {
                log::warn!("[Like] No token available for unlike");
                self.toast_manager.show_error("Not authenticated");
            }
        } else {
            // Like the track
            log::info!("[Like] Liking track {}", track_id);
            self.liked_track_ids.insert(track_id);
            
            // Show optimistic toast
            self.toast_manager.show_success("Added to Liked tracks");
            
            // Spawn background task to like via API
            if let Some(token) = self.app_state.get_token() {
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        match crate::api::likes::like_track(&token, track_id).await {
                            Ok(_) => log::info!("[Like] Successfully liked track {}", track_id),
                            Err(e) => log::error!("[Like] Failed to like track {}: {}", track_id, e),
                        }
                    });
                });
            } else {
                log::warn!("[Like] No token available for like");
                self.toast_manager.show_error("Not authenticated");
            }
        }
    }
    
    /// Toggle like status for a playlist by ID
    pub fn toggle_playlist_like(&mut self, playlist_id: u64) {
        let is_liked = self.liked_playlist_ids.contains(&playlist_id);
        
        if is_liked {
            // Unlike the playlist
            log::info!("[Like] Unliking playlist {}", playlist_id);
            self.liked_playlist_ids.remove(&playlist_id);
            
            // Show optimistic toast
            self.toast_manager.show_info("Removed from Liked playlists");
            
            // Spawn background task to unlike via API
            if let Some(token) = self.app_state.get_token() {
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        match crate::api::likes::unlike_playlist(&token, playlist_id).await {
                            Ok(_) => log::info!("[Like] Successfully unliked playlist {}", playlist_id),
                            Err(e) => log::error!("[Like] Failed to unlike playlist {}: {}", playlist_id, e),
                        }
                    });
                });
            } else {
                log::warn!("[Like] No token available for unlike playlist");
                self.toast_manager.show_error("Not authenticated");
            }
        } else {
            // Like the playlist
            log::info!("[Like] Liking playlist {}", playlist_id);
            self.liked_playlist_ids.insert(playlist_id);
            
            // Show optimistic toast
            self.toast_manager.show_success("Added to Liked playlists");
            
            // Spawn background task to like via API
            if let Some(token) = self.app_state.get_token() {
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        match crate::api::likes::like_playlist(&token, playlist_id).await {
                            Ok(_) => log::info!("[Like] Successfully liked playlist {}", playlist_id),
                            Err(e) => log::error!("[Like] Failed to like playlist {}: {}", playlist_id, e),
                        }
                    });
                });
            } else {
                log::warn!("[Like] No token available for like playlist");
                self.toast_manager.show_error("Not authenticated");
            }
        }
    }
    
    /// Handle keyboard shortcuts (all require Ctrl modifier to avoid interfering with text input)
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Ctrl+Space: Play/Pause
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Space) {
                if self.is_playing {
                    self.audio_controller.pause();
                    self.is_playing = false;
                } else if self.current_track_id.is_some() {
                    self.audio_controller.resume();
                    self.is_playing = true;
                }
            }
            
            // Ctrl+L: Toggle like (navigation Ctrl+L handled in layout.rs)
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::L) {
                self.toggle_current_track_like();
            }
            
            // Ctrl+Shift+S: Toggle shuffle (Ctrl+S is Suggestions navigation)
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S) {
                self.shuffle_mode = !self.shuffle_mode;
                self.playback_queue.set_shuffle(self.shuffle_mode);
                self.save_playback_config();
                let msg = if self.shuffle_mode { "Shuffle on" } else { "Shuffle off" };
                self.toast_manager.show_info(msg);
            }
            
            // Ctrl+Shift+R: Cycle repeat mode (Ctrl+R is Search Results navigation)
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::R) {
                use crate::app_state::RepeatMode;
                self.repeat_mode = match self.repeat_mode {
                    RepeatMode::None => RepeatMode::All,
                    RepeatMode::All => RepeatMode::One,
                    RepeatMode::One => RepeatMode::None,
                };
                self.save_playback_config();
                let msg = match self.repeat_mode {
                    RepeatMode::None => "Repeat off",
                    RepeatMode::All => "Repeat all",
                    RepeatMode::One => "Repeat one",
                };
                self.toast_manager.show_info(msg);
            }
            
            // Ctrl+Arrow Up: Volume up
            if i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowUp) {
                let new_volume = (self.volume + 0.1).min(1.0);
                self.volume = new_volume;
                self.audio_controller.set_volume(new_volume);
                if self.muted {
                    self.muted = false;
                    self.audio_controller.set_volume(new_volume);
                }
            }
            
            // Ctrl+Arrow Down: Volume down
            if i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowDown) {
                let new_volume = (self.volume - 0.1).max(0.0);
                self.volume = new_volume;
                self.audio_controller.set_volume(new_volume);
            }
            
            // Ctrl+Arrow Right: Seek forward 10s
            if i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowRight) && self.current_track_id.is_some() {
                let current_pos = self.audio_controller.get_position();
                let new_pos = current_pos + Duration::from_secs(10);
                if new_pos < Duration::from_millis(self.current_duration_ms) {
                    self.seek_target_pos = Some(new_pos);
                }
            }
            
            // Ctrl+Arrow Left: Seek backward 10s
            if i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowLeft) && self.current_track_id.is_some() {
                let current_pos = self.audio_controller.get_position();
                let new_pos = current_pos.saturating_sub(Duration::from_secs(10));
                self.seek_target_pos = Some(new_pos);
            }
        });
    }

    /// Share current track (copy URL to clipboard)
    pub fn share_current_track(&mut self) {
        let success = crate::utils::clipboard::share_track_url(self.current_permalink_url.as_deref());
        
        if success {
            self.toast_manager.show_success("Track URL copied to clipboard!");
        } else {
            self.toast_manager.show_error("Failed to copy URL - no track playing");
        }
    }

    /// Fetch user info (avatar and username) from /me endpoint
    pub fn fetch_user_info(&mut self) {
        if self.user_avatar_rx.is_some() {
            return; // Already fetching
        }

        // Use token helper to ensure fresh token
        let oauth = match &self.oauth_manager {
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
        self.user_avatar_rx = Some(rx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = crate::utils::http::client();
                
                if let Ok(resp) = client.get("https://api.soundcloud.com/me")
                    .header("Authorization", format!("OAuth {}", token))
                    .send()
                    .await
                {
                    if let Ok(user_json) = resp.json::<serde_json::Value>().await {
                        debug!("Received user data: username={}, avatar={}", 
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
                                            pixels.as_slice()
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
        if let Some(rx) = &self.user_avatar_rx {
            if let Ok(color_image) = rx.try_recv() {
                self.user_avatar_texture = Some(ctx.load_texture(
                    "user_avatar",
                    color_image,
                    egui::TextureOptions::LINEAR
                ));
                self.user_avatar_rx = None;
            }
        }
    }
    
    /// Check for playlist chunk updates (progressive loading)
    pub fn check_playlist_chunks(&mut self) {
        if let Some(rx) = &self.playlist_chunk_rx {
            if let Ok(chunk_tracks) = rx.try_recv() {
                if chunk_tracks.is_empty() {
                    // Empty chunk signals completion
                    log::info!("[App] Playlist loading complete");
                    self.playlist_chunk_rx = None;
                    self.playlist_loading_id = None;
                } else {
                    let chunk_size = chunk_tracks.len();
                    log::info!("[App] Received chunk with {} tracks", chunk_size);
                    
                    // Check if queue was empty (first chunk for new playlist)
                    let is_first_chunk = self.playback_queue.original_tracks.is_empty();
                    
                    // Filter out non-playable tracks (geo-blocked, preview-only, non-streamable)
                    // Note: Database tracks (streamable but no stream_url) are kept and fetched on-demand
                    let playable_tracks: Vec<_> = chunk_tracks.into_iter()
                        .filter(|t| {
                            // Keep streamable tracks
                            if !t.streamable.unwrap_or(false) {
                                return false;
                            }
                            
                            // Filter geo-blocked tracks
                            if let Some(policy) = &t.policy {
                                if policy.to_uppercase() == "BLOCK" {
                                    log::debug!("[Chunk] Filtering geo-locked: {}", t.title);
                                    return false;
                                }
                            }
                            
                            // Filter preview-only/blocked tracks
                            if let Some(access) = &t.access {
                                let access_lower = access.to_lowercase();
                                if access_lower == "blocked" || access_lower == "preview" {
                                    log::debug!("[Chunk] Filtering restricted access: {}", t.title);
                                    return false;
                                }
                            }
                            
                            true
                        })
                        .collect();
                    
                    if !playable_tracks.is_empty() {
                        let filtered_count = playable_tracks.len();
                        if filtered_count < chunk_size {
                            log::info!("[App] Filtered {} → {} playable tracks", 
                                       chunk_size, filtered_count);
                        }
                        
                        // Append tracks to existing queue
                        self.playback_queue.append_tracks(playable_tracks.clone());
                        log::info!("[App] Added {} tracks to queue (total: {})", 
                                   filtered_count, 
                                   self.playback_queue.original_tracks.len());
                        
                        // If this was the first chunk, start playback
                        if is_first_chunk {
                            if let Some(first_track) = self.playback_queue.current_track() {
                                let track_id = first_track.id;
                                log::info!("[App] Starting playback with first chunk");
                                self.play_track(track_id);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Logout user
    pub fn logout(&mut self) {
        // Clear token
        self.app_state.clear_token();
        
        // Clear queue and playback state
        self.playback_queue = PlaybackQueue::new();
        self.current_track_id = None;
        self.current_title = String::new();
        self.current_artist = String::new();
        self.current_genre = None;
        self.current_duration_ms = 0;
        self.current_stream_url = None;
        self.track_start_time = None;
        
        // Clear user info
        self.user_avatar_texture = None;
        self.user_avatar_url = None;
        self.user_username = None;
        
        // Clear search results
        self.search_query.clear();
        self.search_results_tracks.clear();
        self.search_results_playlists.clear();
        self.search_loading = false;
        
        // Clear artwork cache
        self.artwork_texture = None;
        self.artwork_loading = false;
        self.thumb_cache.clear();
        self.thumb_pending.clear();
        
        // Stop playback
        self.audio_controller.stop();
        self.is_playing = false;
        
        // Reset tab to Home
        self.selected_tab = MainTab::Home;
        
        // Return to splash screen for re-login
        self.screen = AppScreen::Splash;
    }

    /// Check if track finished and handle auto-play
    pub fn check_track_finished(&mut self) {
        // Only check for track completion if we're currently playing (not paused)
        // This prevents false positives when sink is empty due to pause state
        if self.is_playing && self.audio_controller.is_finished() {
            // Additional check: ensure we have a valid track and it's actually started
            // track_start_time is set after audio successfully loads, so this prevents
            // false positives during the loading phase
            if self.current_track_id.is_none() || self.track_start_time.is_none() {
                return;
            }
            
            // Prevent race condition: don't treat as finished if track just started (< 1 second ago)
            if let Some(start_time) = self.track_start_time {
                if start_time.elapsed() < Duration::from_secs(1) {
                    return;
                }
            }
            
            log::info!("Track finished, handling auto-play/stop");
            
            match self.repeat_mode {
                RepeatMode::One => {
                    // Replay current track
                    if let Some(track_id) = self.current_track_id {
                        info!("Repeat One: replaying track {}", track_id);
                        self.play_track(track_id);
                    }
                }
                RepeatMode::All => {
                    // Check if we're at the end of the queue
                    let at_end = self.playback_queue.current_index
                        .map(|idx| idx >= self.playback_queue.current_queue.len() - 1)
                        .unwrap_or(true);
                    
                    if at_end {
                        // Loop back to first track
                        info!("Repeat All: looping back to first track");
                        if let Some(first_track) = self.playback_queue.original_tracks.first() {
                            self.playback_queue.current_index = Some(0);
                            self.play_track(first_track.id);
                        }
                    } else {
                        self.play_next();
                    }
                }
                RepeatMode::None => {
                    // Just play next, stop if at end
                    let can_play_next = self.playback_queue.current_index
                        .map(|idx| idx < self.playback_queue.current_queue.len() - 1)
                        .unwrap_or(false);
                    
                    if can_play_next {
                        self.play_next();
                    } else {
                        // Check if this was single-track playback
                        let is_single_track = self.playback_queue.current_queue.len() == 1;
                        
                        if is_single_track {
                            // CRITICAL: Don't trigger fetch if one is already in progress
                            if self.track_fetch_rx.is_some() {
                                return; // Already fetching next track, wait for it
                            }
                            
                            // Try to play random track from history (excluding current track)
                            info!("Single track finished, picking random track from history");
                            
                            if let Some(current_id) = self.current_track_id {
                                match crate::utils::playback_history::PlaybackHistoryDB::new() {
                                    Ok(db) => {
                                        // Fetch recent tracks (we'll filter out current one)
                                        let recent = db.get_recent_tracks(50);
                                        
                                        // Filter out current track
                                        let candidates: Vec<_> = recent.iter()
                                            .filter(|r| r.track_id != current_id)
                                            .collect();
                                        
                                        // If we have candidates in history, pick random
                                        if !candidates.is_empty() {
                                            use rand::Rng;
                                            let mut rng = rand::rng();
                                            let random_idx = rng.random_range(0..candidates.len());
                                            let next_record = candidates[random_idx];
                                            
                                            info!("Randomly selected track from history: {} (ID: {})", next_record.title, next_record.track_id);
                                            
                                            // Fetch full track data and play (like History screen does)
                                            self.fetch_and_play_track(next_record.track_id);
                                            return; // Don't stop playback
                                        } else {
                                            // Not enough history, try suggestions instead
                                            info!("Not enough history (< 2 tracks), falling back to suggestions");
                                            
                                            if !self.suggestions_tracks.is_empty() {
                                                // Play first suggestion
                                                let next_track = self.suggestions_tracks[0].clone();
                                                info!("Playing first suggestion: {}", next_track.title);
                                                
                                                self.playback_queue.load_tracks(vec![next_track]);
                                                if let Some(track) = self.playback_queue.current_track() {
                                                    self.play_track(track.id);
                                                }
                                                return; // Don't stop playback
                                            } else {
                                                info!("No suggestions available, stopping playback");
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
                        self.is_playing = false;
                        self.audio_controller.stop();
                        self.last_playback_error = None;
                        // Reset progress to allow new track selection
                        self.track_start_time = None;
                    }
                }
            }
        }
    }

    /// Check for search results from background tasks
    pub fn check_search_results(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.search_rx {
            if let Ok(results) = rx.try_recv() {
                self.search_loading = false;

                // Replace old page with new page
                self.search_results_tracks = results.tracks;
                self.search_results_playlists = results.playlists;

                self.search_next_href = results.next_href.clone();
                self.search_has_more = results.next_href.is_some();

                // Consumed the message, receiver is done
                self.search_rx = None;

                ctx.request_repaint();
            }
        }
    }

    /// Check for playlist load completion from background tasks
    pub fn check_playlist_load(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.playlist_rx {
            if let Ok(playlist) = rx.try_recv() {
                log::info!(
                    "[Playlist] Background load complete: {} total tracks",
                    playlist.tracks.len()
                );

                self.selected_playlist_id = Some(playlist.id);

                let streamable_tracks: Vec<_> = playlist
                    .tracks
                    .into_iter()
                    .filter(|t| t.streamable.unwrap_or(false) && t.stream_url.is_some())
                    .collect();

                if !streamable_tracks.is_empty() {
                    // When loading from Playlists screen, replace the queue (don't merge)
                    log::info!("[Playlist] Loading {} tracks into queue", streamable_tracks.len());
                    
                    // Load playlist into queue (this replaces existing queue)
                    self.playback_queue.load_tracks(streamable_tracks);

                    // Start playing first track
                    if let Some(track) = self.playback_queue.current_track() {
                        log::info!("[Playlist] Playing first track: {}", track.title);
                        self.play_track(track.id);
                    }

                    ctx.request_repaint();
                }

                self.playlist_rx = None;
            }
        }
    }

    /// Check for home screen data updates from background tasks
    pub fn check_home_updates(&mut self) {
        // Check recently played
        if let Some(rx) = &self.home_recently_played_rx {
            if let Ok(tracks) = rx.try_recv() {
                let track_count = tracks.len();
                log::info!("[Home] Received {} recently played tracks", track_count);
                self.home_content.recently_played = tracks.clone();
                self.home_content.initial_fetch_done = true;
                self.home_loading = false;
                self.home_recently_played_rx = None;
                
                // Only fetch recommendations if we have history to base them on
                if !tracks.is_empty() && !self.home_recommendations_loading {
                    // Fetch 6 recommendations based on recently played
                    self.fetch_recommendations(tracks, 6);
                }
            }
        }
        
        // Check recommendations
        if let Some(rx) = &self.home_recommendations_rx {
            if let Ok(mut tracks) = rx.try_recv() {
                log::info!("[Home] Received {} recommended tracks", tracks.len());
                
                // If we have less than 6, fill with history tracks
                if tracks.len() < 6 {
                    let needed = 6 - tracks.len();
                    log::info!("[Home] Filling {} empty slots with history tracks", needed);
                    
                    // Get history tracks that aren't already in recommendations
                    let rec_ids: std::collections::HashSet<u64> = tracks.iter().map(|t| t.id).collect();
                    let history_tracks: Vec<_> = self.home_content.recently_played.iter()
                        .filter(|t| !rec_ids.contains(&t.id))
                        .take(needed)
                        .cloned()
                        .collect();
                    
                    tracks.extend(history_tracks);
                }
                
                // Store recommendations (max 6)
                self.home_content.recommendations = tracks.into_iter().take(6).collect();
                self.home_recommendations_loading = false;
                self.home_recommendations_rx = None;
            }
        }
        
        // Check suggestions
        if let Some(rx) = &self.suggestions_rx {
            if let Ok(mut tracks) = rx.try_recv() {
                log::info!("[Suggestions] Received {} suggestion tracks", tracks.len());
                
                // If we have less than 12, fill with history tracks
                if tracks.len() < 12 {
                    let needed = 12 - tracks.len();
                    log::info!("[Suggestions] Filling {} empty slots with history tracks", needed);
                    
                    // Get history tracks that aren't already in suggestions
                    let sug_ids: std::collections::HashSet<u64> = tracks.iter().map(|t| t.id).collect();
                    let history_records = self.playback_history.get_recent_tracks(needed + 10);
                    let history_tracks: Vec<_> = history_records.iter()
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
                self.suggestions_tracks = tracks;
                self.suggestions_loading = false;
                self.suggestions_rx = None;
                self.suggestions_initial_fetch_done = true;
            }
        }
    }
    
    /// Fetch home screen data (recently played from local database)
    pub fn fetch_home_data(&mut self) {
        if self.home_loading {
            return; // Already loading
        }
        
        log::info!("[Home] Fetching recently played tracks from local database (ordered by played_at DESC)...");
        self.home_loading = true;
        
        let (tx, rx) = channel();
        self.home_recently_played_rx = Some(rx);
        
        // Fetch directly from database - no queue needed
        let token = self.oauth_manager.as_ref()
            .and_then(|oauth| crate::utils::token_helper::get_valid_token_sync(oauth))
            .map(|t| t.access_token.clone())
            .unwrap_or_default();
        crate::app::home::fetch_recently_played_async(token, tx);
    }
    
    /// Refresh recently played section immediately (after new track starts)
    fn refresh_home_recently_played(&mut self) {
        log::info!("[Home] Refreshing recently played and recommendations after track change...");
        
        // First, get the current track from queue to use for recommendations
        let current_track = self.playback_queue.current_track().cloned();
        
        if let Some(track) = current_track {
            // Immediately fetch recommendations based on newly playing track
            if let Some(oauth) = &self.oauth_manager {
                if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                    if !self.home_recommendations_loading {
                        log::info!("[Home] Fetching recommendations for newly playing track...");
                        self.home_recommendations_loading = true;
                        
                        let (rec_tx, rec_rx) = channel();
                        self.home_recommendations_rx = Some(rec_rx);
                        
                        // Fetch recommendations immediately
                        crate::app::home::fetch_recommendations_async(
                            token_data.access_token,
                            vec![track],
                            rec_tx,
                            5
                        );
                    }
                }
            }
        }
        
        // Then refresh recently played list from database (ordered by played_at DESC)
        let (tx, rx) = channel();
        self.home_recently_played_rx = Some(rx);
        
        // Fetch directly from database - no queue needed
        let token = self.oauth_manager.as_ref()
            .and_then(|oauth| crate::utils::token_helper::get_valid_token_sync(oauth))
            .map(|t| t.access_token.clone())
            .unwrap_or_default();
        crate::app::home::fetch_recently_played_async(token, tx);
    }
    
    /// Fetch recommendations based on recently played tracks
    fn fetch_recommendations(&mut self, recently_played: Vec<crate::app::playlists::Track>, limit: usize) {
        if self.home_recommendations_loading {
            return;
        }
        
        if let Some(oauth) = &self.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Home] Fetching {} recommendations...", limit);
                self.home_recommendations_loading = true;
                
                let (tx, rx) = channel();
                self.home_recommendations_rx = Some(rx);
                
                crate::app::home::fetch_recommendations_async(
                    token_data.access_token,
                    recently_played,
                    tx,
                    limit
                );
            }
        }
    }

    /// Fetch all suggestions for the Suggestions screen (up to 100 tracks)
    pub fn fetch_all_suggestions(&mut self) {
        if self.suggestions_loading {
            return;
        }
        
        // Get recently played tracks to base suggestions on
        let recent_tracks = self.playback_history.get_recent_tracks(50);
        if recent_tracks.is_empty() {
            log::info!("[Suggestions] No playback history, skipping suggestions fetch");
            self.suggestions_initial_fetch_done = true;
            return;
        }
        
        // Convert to Track objects (simple version, we just need IDs)
        let tracks: Vec<crate::app::playlists::Track> = recent_tracks.iter().map(|record| {
            crate::app::playlists::Track {
                id: record.track_id,
                title: record.title.clone(),
                user: crate::app::playlists::User {
                    id: 0,
                    username: record.artist.clone(),
                    avatar_url: None,
                },
                duration: record.duration,
                genre: record.genre.clone(),
                artwork_url: None,
                permalink_url: None,
                stream_url: None,
                streamable: Some(true),
                playback_count: None,
                access: None,
                policy: None,
            }
        }).collect();
        
        if let Some(oauth) = &self.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Suggestions] Fetching 12 suggestions based on {} recent tracks...", tracks.len());
                self.suggestions_loading = true;
                
                let (tx, rx) = channel();
                self.suggestions_rx = Some(rx);
                
                crate::app::home::fetch_recommendations_async(
                    token_data.access_token,
                    tracks,
                    tx,
                    12  // Fetch 12 suggestions to match page size
                );
            }
        }
    }

    /// Fetch user's liked tracks and uploaded tracks
    pub fn fetch_likes(&mut self) {
        if self.likes_loading {
            return;
        }
        
        if let Some(oauth) = &self.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Likes] Fetching liked tracks and user tracks...");
                self.likes_loading = true;
                
                let token = token_data.access_token.clone();
                
                // Fetch liked tracks
                let (tracks_tx, tracks_rx) = channel();
                self.likes_tracks_rx = Some(tracks_rx);
                
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
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
                self.user_tracks_rx = Some(user_rx);
                
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
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
        if let Some(oauth) = &self.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Likes] Fetching liked track IDs for social buttons...");
                
                let token = token_data.access_token.clone();
                let (tx, rx) = channel();
                self.likes_tracks_rx = Some(rx);
                
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
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
        if self.playlists_loading {
            return;
        }
        
        if let Some(oauth) = &self.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Playlists] Fetching user playlists...");
                self.playlists_loading = true;
                
                let token = token_data.access_token.clone();
                let (playlists_tx, playlists_rx): (_, Receiver<(Vec<_>, Vec<u64>)>) = channel();
                self.playlists_rx = Some(playlists_rx);
                
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        match crate::api::likes::fetch_user_playlists(&token).await {
                            Ok((playlists, created_ids)) => {
                                log::info!("[Playlists] Fetched {} playlists ({} created)", playlists.len(), created_ids.len());
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
        if let Some(rx) = &self.likes_tracks_rx {
            if let Ok(tracks) = rx.try_recv() {
                log::info!("[Likes] Received {} liked tracks", tracks.len());
                
                // Update liked track IDs HashSet
                self.liked_track_ids.clear();
                for track in &tracks {
                    self.liked_track_ids.insert(track.id);
                }
                log::info!("[Likes] Updated liked_track_ids with {} IDs", self.liked_track_ids.len());
                
                self.likes_tracks = tracks;
                self.likes_tracks_rx = None;
            } else {
                pending += 1;
            }
        }
        
        // Check user uploaded tracks
        if let Some(rx) = &self.user_tracks_rx {
            if let Ok(tracks) = rx.try_recv() {
                log::info!("[Likes] Received {} user uploaded tracks", tracks.len());
                self.user_tracks = tracks;
                self.user_tracks_rx = None;
            } else {
                pending += 1;
            }
        }
        
        // Mark loading as complete when all channels are done
        if pending == 0 {
            self.likes_loading = false;
        }
    }
    
    /// Check for playlists updates from background tasks
    pub fn check_playlists_updates(&mut self) {
        if let Some(rx) = &self.playlists_rx {
            if let Ok((playlists, created_ids)) = rx.try_recv() {
                log::info!("[Playlists] Received {} playlists ({} created)", playlists.len(), created_ids.len());
                
                // Track user-created playlist IDs
                self.user_created_playlist_ids.clear();
                for id in created_ids {
                    self.user_created_playlist_ids.insert(id);
                }
                
                // Build liked playlist IDs set (all playlists)
                self.liked_playlist_ids.clear();
                for playlist in &playlists {
                    self.liked_playlist_ids.insert(playlist.id);
                }
                
                self.playlists = playlists;
                self.playlists_rx = None;
                self.playlists_loading = false;
            }
        }
    }

    /// Fetch popular tracks for new users with no activity (fallback)
    /// Check if token has expired and trigger re-authentication if needed
    pub fn check_token_expiry(&mut self) {
        let now = Instant::now();
        
        // Check every 60 seconds
        if let Some(last_check) = self.last_token_check {
            if now.duration_since(last_check) < self.token_check_interval {
                return; // Not time to check yet
            }
        }
        
        self.last_token_check = Some(now);
        
        // Only check if we're on the main screen (logged in)
        if !matches!(self.screen, AppScreen::Main) {
            return;
        }
        
        // Check and refresh token if needed using helper
        if let Some(oauth) = &self.oauth_manager {
            // Don't do anything if refresh is already in progress
            if self.refresh_in_progress {
                log::debug!("[OAuth] Refresh already in progress, waiting...");
                return;
            }
            
            // Mark refresh as in progress
            self.refresh_in_progress = true;
            
            let oauth_clone = oauth.clone();
            
            // Spawn refresh task in background
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let _ = crate::utils::token_helper::ensure_fresh_token(&oauth_clone).await;
                });
            });
        }
    }
    
    /// Fetch track data from API and play it (for database tracks with no stream_url)
    pub fn fetch_and_play_track(&mut self, track_id: u64) {
        if let Some(oauth) = &self.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Home] Fetching full track data for ID: {}", track_id);
                
                let token = token_data.access_token.clone();
                let (tx, rx) = channel();
                
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        match crate::app::playlists::fetch_track_by_id(&token, track_id).await {
                            Ok(track) => {
                                log::info!("[Home] Fetched track: {}", track.title);
                                let _ = tx.send(Ok(vec![track]));
                            }
                            Err(e) => {
                                let err_msg = e.to_string();
                                // Check if it's a "not playable" or "restricted" error - treat as warning, not fatal
                                if err_msg.contains("not playable") || err_msg.contains("not available") || err_msg.contains("restricted") {
                                    log::warn!("[Home] Skipping unavailable track {}: {}", track_id, e);
                                    let _ = tx.send(Ok(vec![])); // Return empty instead of error - triggers auto-skip
                                } else {
                                    log::error!("[Home] Failed to fetch track {}: {}", track_id, e);
                                    let _ = tx.send(Err(err_msg));
                                }
                            }
                        }
                    });
                });
                
                // Store receiver for checking in update loop
                self.track_fetch_rx = Some(rx);
            }
        }
    }
    
    /// Check for fetched track data and play when ready
    fn check_track_fetch(&mut self) {
        if let Some(rx) = &self.track_fetch_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(tracks) => {
                        if !tracks.is_empty() {
                            log::info!("[Home] Track(s) fetched, loading into queue");
                            self.playback_queue.load_tracks(tracks.clone());
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
                        self.last_playback_error = Some(format!("Failed to load track: {}", e));
                    }
                }
                self.track_fetch_rx = None;
            }
        }
    }
    
    /// Fetch multiple tracks from API and play as playlist
    pub fn fetch_and_play_playlist(&mut self, track_ids: Vec<u64>) {
        if let Some(oauth) = &self.oauth_manager {
            if let Some(token_data) = crate::utils::token_helper::get_valid_token_sync(oauth) {
                log::info!("[Home] Fetching {} tracks from API...", track_ids.len());
                
                let token = token_data.access_token.clone();
                let (tx, rx) = channel();
                
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let mut tracks = Vec::new();
                        for track_id in track_ids {
                            match crate::app::playlists::fetch_track_by_id(&token, track_id).await {
                                Ok(track) => tracks.push(track),
                                Err(e) => log::warn!("[Home] Skipping track {}: {}", track_id, e),
                            }
                        }
                        if !tracks.is_empty() {
                            let _ = tx.send(Ok(tracks));
                        } else {
                            let _ = tx.send(Err("No playable tracks found".to_string()));
                        }
                    });
                });
                
                self.track_fetch_rx = Some(rx);
            }
        }
    }
}

impl eframe::App for MusicPlayerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Handle close request - cleanup and exit immediately
        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.is_shutting_down {
                self.is_shutting_down = true;
                self.cleanup_and_exit(ctx, frame);
            }
        }
        // Handle OAuth authentication flow and token validation
        if matches!(self.screen, AppScreen::Splash) {
            // Check for existing valid token (only once per session)
            if !self.token_check_done {
                self.token_check_done = true;
                
                if let Some(oauth_manager) = &self.oauth_manager {
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
            let has_valid_token = if let Some(oauth_manager) = &self.oauth_manager {
                oauth_manager.get_token().is_some()
            } else {
                false
            };
            
            if has_valid_token {
                // Check if minimum splash duration has elapsed
                let can_transition = if let Some(start_time) = self.splash_start_time {
                    start_time.elapsed() >= self.splash_min_duration
                } else {
                    true // If no timer, allow immediate transition
                };
                
                if can_transition {
                    log::info!("[Splash] Minimum display time elapsed, transitioning to main screen");
                    self.is_authenticating = false;
                    
                    // Clean up splash shader resources (will be lazy loaded if needed again)
                    if self.splash_shader.is_some() {
                        log::info!("[Shader] Releasing splash shader resources");
                        self.splash_shader = None;
                    }
                    
                    self.screen = AppScreen::Main;
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
        if matches!(self.screen, AppScreen::Main) {
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
        
        // Check for search results (background tasks)
        self.check_search_results(ctx);
        
        // Check for playlist load completion
        self.check_playlist_load(ctx);
        
        // Check for home screen data updates
        self.check_home_updates();
        
        // Check for fetched track data (from database tracks)
        self.check_track_fetch();

        // Check if token has expired (every 60 seconds)
        self.check_token_expiry();

        // Check if track finished for auto-play
        if matches!(self.screen, AppScreen::Main) {
            self.check_track_finished();
        }

        match self.screen {
            AppScreen::Splash => {
                crate::screens::render_splash_screen(self, ctx);
            }
            AppScreen::Main => {
                // AUDIO REACTIVITY: Use real FFT analysis
                if self.is_playing {
                    // Read bass energy for overall amplitude (pulsing effect)
                    if let Ok(bass) = self.bass_energy.lock() {
                        self.audio_amplitude = *bass;
                    }
                } else {
                    self.audio_amplitude = 0.0;
                }
                
                crate::ui_components::layout::render_with_layout(self, ctx);
            }
        }
        
        // Render toasts on top of everything
        egui::Area::new(egui::Id::new("toast_area"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                self.toast_manager.render(ui);
            });

        // Optimized repaint: only request when playing, loading, or toasts active
        if self.is_playing 
            || self.search_loading 
            || self.home_loading 
            || !self.toast_manager.toasts.is_empty() 
            || self.artwork_loading
        {
            ctx.request_repaint();
        }
    }
}

