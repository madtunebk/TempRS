use std::sync::{Arc, RwLock};

/// Centralized application state that can be shared across modules
#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<AppStateInner>>,
}

#[allow(dead_code)]
struct AppStateInner {
    /// When the current token expires (Unix timestamp)
    pub token_expires_at: Option<u64>,
    
    /// Current user display name
    pub user_display_name: Option<String>,
    
    /// Is the app currently authenticated?
    pub is_authenticated: bool,
    
    /// Application version
    pub app_version: String,
    
    /// Playback settings (in-memory, not persisted)
    pub volume: f32,
    pub muted: bool,
    pub shuffle_mode: bool,
    pub repeat_mode: RepeatMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RepeatMode {
    None,
    One,
    All,
}

#[allow(dead_code)]
impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AppStateInner {
                token_expires_at: None,
                user_display_name: None,
                is_authenticated: false,
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                volume: 0.5,
                muted: false,
                shuffle_mode: false,
                repeat_mode: RepeatMode::None,
            })),
        }
    }
    
    /// Set token expiration time
    pub fn set_token_expires_at(&self, expires_at: u64) {
        if let Ok(mut state) = self.inner.write() {
            state.token_expires_at = Some(expires_at);
        }
    }
    
    /// Get token expiration time
    pub fn get_token_expires_at(&self) -> Option<u64> {
        self.inner.read().ok()?.token_expires_at
    }
    
    /// Check if token is still valid
    pub fn is_token_valid(&self) -> bool {
        if let Some(expires_at) = self.get_token_expires_at() {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            expires_at > current_time
        } else {
            false
        }
    }
    
    /// Set user display name
    pub fn set_user_display_name(&self, name: String) {
        if let Ok(mut state) = self.inner.write() {
            state.user_display_name = Some(name);
        }
    }
    
    /// Get user display name
    pub fn get_user_display_name(&self) -> Option<String> {
        self.inner.read().ok()?.user_display_name.clone()
    }
    
    /// Set authentication status
    pub fn set_authenticated(&self, authenticated: bool) {
        if let Ok(mut state) = self.inner.write() {
            state.is_authenticated = authenticated;
        }
    }
    
    /// Check if user is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.inner.read().ok().map_or(false, |s| s.is_authenticated)
    }
    
    /// Get app version
    pub fn get_app_version(&self) -> String {
        self.inner.read().ok()
            .map(|s| s.app_version.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }
    
    /// Clear all state (on logout)
    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(mut state) = self.inner.write() {
            state.token_expires_at = None;
            state.user_display_name = None;
            state.is_authenticated = false;
        }
    }
    
    // Playback settings
    
    pub fn set_volume(&self, volume: f32) {
        if let Ok(mut state) = self.inner.write() {
            state.volume = volume.clamp(0.0, 1.0);
        }
    }
    
    pub fn get_volume(&self) -> f32 {
        self.inner.read().ok().map_or(0.5, |s| s.volume)
    }
    
    pub fn set_muted(&self, muted: bool) {
        if let Ok(mut state) = self.inner.write() {
            state.muted = muted;
        }
    }
    
    pub fn is_muted(&self) -> bool {
        self.inner.read().ok().map_or(false, |s| s.muted)
    }
    
    pub fn set_shuffle_mode(&self, shuffle: bool) {
        if let Ok(mut state) = self.inner.write() {
            state.shuffle_mode = shuffle;
        }
    }
    
    pub fn get_shuffle_mode(&self) -> bool {
        self.inner.read().ok().map_or(false, |s| s.shuffle_mode)
    }
    
    pub fn set_repeat_mode(&self, mode: RepeatMode) {
        if let Ok(mut state) = self.inner.write() {
            state.repeat_mode = mode;
        }
    }
    
    pub fn get_repeat_mode(&self) -> RepeatMode {
        self.inner.read().ok().map_or(RepeatMode::None, |s| s.repeat_mode)
    }
    
    /// Get access token from database
    pub fn get_token(&self) -> Option<String> {
        let token_store = crate::utils::token_store::TokenStore::new();
        token_store.get_valid_token().map(|t| t.access_token)
    }
    
    /// Clear token from database
    pub fn clear_token(&self) {
        let token_store = crate::utils::token_store::TokenStore::new();
        let _ = token_store.delete_token();
        
        // Also clear authentication state
        if let Ok(mut state) = self.inner.write() {
            state.is_authenticated = false;
            state.token_expires_at = None;
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
