use crate::utils::oauth::OAuthManager;
use std::time::{Duration, Instant};

pub struct AuthState {
    pub oauth_manager: Option<OAuthManager>,
    pub is_authenticating: bool,
    pub login_message_shown: bool,
    pub refresh_attempted: bool,
    pub token_check_done: bool,
    pub refresh_in_progress: bool,
    pub user_avatar_url: Option<String>,
    pub user_avatar_texture: Option<egui::TextureHandle>,
    pub user_username: Option<String>,
    #[allow(dead_code)]
    pub show_user_menu: bool,
    pub last_token_check: Option<Instant>,
    pub token_check_interval: Duration,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            oauth_manager: {
                use crate::utils::oauth::OAuthConfig;
                let client_id = crate::SOUNDCLOUD_CLIENT_ID.to_string();
                let client_secret = crate::SOUNDCLOUD_CLIENT_SECRET.to_string();
                let redirect_uri = "http://localhost:3000/callback".to_string();
                let config = OAuthConfig::new(client_id, client_secret, redirect_uri);
                Some(OAuthManager::new(config))
            },
            is_authenticating: false,
            login_message_shown: false,
            refresh_attempted: false,
            token_check_done: false,
            refresh_in_progress: false,
            user_avatar_url: None,
            user_avatar_texture: None,
            user_username: None,
            show_user_menu: false,
            last_token_check: None,
            token_check_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl AuthState {
    /// Check if user is authenticated (has valid token)
    #[allow(dead_code)]
    pub fn is_authenticated(&self) -> bool {
        if let Some(oauth) = &self.oauth_manager {
            if let Some(token_data) = oauth.get_token() {
                // Check if token is expired
                if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                    let current_time = duration.as_secs();
                    return current_time < token_data.expires_at;
                }
            }
        }
        false
    }

    /// Get current access token if valid
    #[allow(dead_code)]
    pub fn get_token(&self) -> Option<String> {
        if let Some(oauth) = &self.oauth_manager {
            if let Some(token_data) = oauth.get_token() {
                // Check if token is expired
                if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                    let current_time = duration.as_secs();
                    if current_time < token_data.expires_at {
                        return Some(token_data.access_token.clone());
                    }
                }
            }
        }
        None
    }

    /// Clear user session (logout)
    pub fn clear_session(&mut self) {
        if let Some(oauth) = &mut self.oauth_manager {
            let _ = oauth.logout(); // Ignore errors during logout
        }
        self.user_avatar_url = None;
        self.user_avatar_texture = None;
        self.user_username = None;
        self.is_authenticating = false;
        self.login_message_shown = false;
        self.refresh_attempted = false;
        self.token_check_done = false;
        self.refresh_in_progress = false;
    }
}
