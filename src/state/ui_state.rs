use crate::app::shader_manager::ShaderManager;
use crate::ui_components::toast::ToastManager;
use egui::{Color32, TextureHandle};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
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

pub struct UIState {
    // Navigation
    pub screen: AppScreen,
    pub selected_tab: MainTab,

    // Visual Assets
    pub logo_texture: Option<TextureHandle>,
    pub no_artwork_texture: Option<TextureHandle>,

    // Toast Notifications
    pub toast_manager: ToastManager,

    // Current Track Artwork & Visuals
    pub artwork_texture: Option<TextureHandle>,
    pub artwork_loading: bool,
    pub artwork_dominant_color: Color32,
    pub artwork_edge_colors: [Color32; 4], // Ambilight: [top, right, bottom, left]

    // Thumbnail Cache (for playlist/search results)
    pub thumb_cache: HashMap<String, TextureHandle>,
    pub thumb_pending: HashMap<String, bool>,

    // Ambient Glow & Audio Reactivity
    #[allow(dead_code)]
    pub glow_intensity: f32,
    #[allow(dead_code)]
    pub glow_smooth_intensity: f32,
    #[allow(dead_code)]
    pub last_frame_time: Option<Instant>,
    pub audio_amplitude: f32, // Real-time audio level (0.0-1.0) for reactive visuals

    // Playback Error Display
    pub last_playback_error: Option<String>,

    // Shader Management
    pub shader_manager: ShaderManager,

    // UI Controls
    pub show_volume_popup: bool,
    #[allow(dead_code)]
    pub show_exit_confirmation: bool,
    pub is_shutting_down: bool,
    pub is_seeking: bool,
    pub seek_target_pos: Option<Duration>,
    pub queue_collapsed: bool,

    // Splash Screen
    pub splash_start_time: Option<Instant>,
    pub splash_min_duration: Duration,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            screen: AppScreen::Splash,
            selected_tab: MainTab::Home,
            logo_texture: None,
            no_artwork_texture: None,
            toast_manager: ToastManager::new(),
            artwork_texture: None,
            artwork_loading: false,
            artwork_dominant_color: Color32::from_rgb(30, 30, 30),
            artwork_edge_colors: [Color32::from_rgb(30, 30, 30); 4],
            thumb_cache: HashMap::new(),
            thumb_pending: HashMap::new(),
            glow_intensity: 0.0,
            glow_smooth_intensity: 0.0,
            last_frame_time: None,
            audio_amplitude: 0.0,
            last_playback_error: None,
            shader_manager: ShaderManager::new(),
            show_volume_popup: false,
            show_exit_confirmation: false,
            is_shutting_down: false,
            is_seeking: false,
            seek_target_pos: None,
            queue_collapsed: false,
            splash_start_time: Some(Instant::now()),
            splash_min_duration: Duration::from_millis(1500),
        }
    }
}

impl UIState {
    /// Check if splash screen should still be shown
    #[allow(dead_code)]
    pub fn is_splash_active(&self) -> bool {
        if let Some(start) = self.splash_start_time {
            start.elapsed() < self.splash_min_duration
        } else {
            false
        }
    }

    /// Transition from splash to main screen
    #[allow(dead_code)]
    pub fn transition_to_main(&mut self) {
        self.screen = AppScreen::Main;
        self.splash_start_time = None;
    }

    /// Update audio amplitude for reactive visuals
    #[allow(dead_code)]
    pub fn update_audio_amplitude(&mut self, bass_energy: f32) {
        self.audio_amplitude = bass_energy;
    }

    /// Update glow intensity with smoothing
    #[allow(dead_code)]
    pub fn update_glow(&mut self, target_intensity: f32, delta_time: f32) {
        let smooth_speed = 3.0;
        self.glow_smooth_intensity +=
            (target_intensity - self.glow_smooth_intensity) * smooth_speed * delta_time;
        self.glow_intensity = self.glow_smooth_intensity;
    }
}
