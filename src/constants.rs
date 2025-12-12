//! Application constants and configuration values

// === UI & Layout ===
pub const GRID_PAGE_SIZE: usize = 12;
pub const SPLASH_MIN_DURATION_SECS: u64 = 2;
pub const SPLASH_CHECK_INTERVAL_MILLIS: u64 = 100;

// Frame rate settings (performance optimization based on renderer)
pub const REPAINT_INTERVAL_GPU_MICROS: u64 = 8333;   // 120 FPS (8.33ms per frame) - GPU mode with shaders
pub const REPAINT_INTERVAL_CPU_ACTIVE: u64 = 33333;  // 30 FPS (33ms per frame) - CPU mode when loading/toasts
pub const REPAINT_INTERVAL_CPU_IDLE: u64 = 50000;    // 20 FPS (50ms per frame) - CPU mode when idle

// === SoundCloud Branding ===
pub const DOMINANT_COLOR_RGB: (u8, u8, u8) = (255, 85, 0); // SoundCloud orange

// === Audio Playback ===
pub const VOLUME_STEP: f32 = 0.1;
pub const DEFAULT_VOLUME_BEFORE_MUTE: f32 = 0.7;
pub const SEEK_STEP_SECS: u64 = 10;
pub const MIN_TRACK_ELAPSED_SECS: u64 = 1;

// === API & Content ===
pub const HOME_RECOMMENDATIONS_LIMIT: usize = 6;
pub const SUGGESTIONS_LIKES_LIMIT: usize = 30;
pub const SUGGESTIONS_USER_TRACKS_LIMIT: usize = 20;

// === OAuth ===
pub const OAUTH_REDIRECT_URI: &str = "http://localhost:3000/callback";
pub const TOKEN_CHECK_INTERVAL_SECS: u64 = 60;
