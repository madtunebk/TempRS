/// Token helper utilities - ensures valid tokens before API calls
///
/// # Usage Examples
///
/// ## From async context (playlists.rs, etc):
/// ```rust
/// use crate::utils::token_helper::get_valid_token;
///
/// pub async fn fetch_something(oauth: &OAuthManager) -> Result<...> {
///     let token = match get_valid_token(oauth).await {
///         Some(t) => t.access_token,
///         None => return Err("No valid token".to_string()),
///     };
///     
///     // Make API call with fresh token
///     let client = reqwest::Client::new();
///     client.get("https://api.soundcloud.com/...")
///         .header("Authorization", format!("OAuth {}", token))
///         .send()
///         .await
/// }
/// ```
///
/// ## From UI thread (player_app.rs):
/// ```rust
/// use crate::utils::token_helper::get_valid_token_sync;
///
/// let oauth = self.oauth_manager.as_ref()?;
/// let token = match get_valid_token_sync(oauth) {
///     Some(t) => t.access_token,
///     None => {
///         log::warn!("No valid token available");
///         return;
///     }
/// };
///
/// // Spawn API call with fresh token
/// std::thread::spawn(move || {
///     // ...
/// });
/// ```
use crate::utils::oauth::OAuthManager;
use crate::utils::token_store::TokenData;
use log::{error, info, warn};

/// Get a valid token, refreshing if necessary
/// Returns None if token is expired and refresh fails (needs re-login)
pub async fn get_valid_token(oauth: &OAuthManager) -> Option<TokenData> {
    // First try to get valid token (not expired)
    if let Some(token) = oauth.get_token() {
        return Some(token);
    }

    // Token is expired or missing - try to refresh
    info!("[TokenHelper] Token expired or missing, attempting refresh...");

    if let Some(expired_token) = oauth.get_token_for_refresh() {
        if let Some(refresh_token) = &expired_token.refresh_token {
            match oauth.refresh_token(refresh_token).await {
                Ok(new_token) => {
                    info!("[TokenHelper] Token refreshed successfully!");
                    return Some(new_token);
                }
                Err(e) => {
                    error!("[TokenHelper] Token refresh failed: {}", e);
                    return None; // Refresh failed - needs re-login
                }
            }
        } else {
            warn!("[TokenHelper] No refresh token available");
        }
    }

    None // No valid token available
}

/// Check if token is about to expire and refresh proactively
/// Returns true if token is valid or was refreshed successfully
pub async fn ensure_fresh_token(oauth: &OAuthManager) -> bool {
    let current_time = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(e) => {
            log::error!(
                "[TokenHelper] System time error (clock before 1970?): {}",
                e
            );
            return false; // Assume token expired to trigger re-auth
        }
    };

    // Get token even if expired (to check expiry time)
    if let Some(token) = oauth.get_token_for_refresh() {
        let time_until_expiry = token.expires_at.saturating_sub(current_time);

        // Refresh if token expires in less than 5 minutes (300 seconds)
        if time_until_expiry < 300 {
            if time_until_expiry == 0 {
                warn!("[TokenHelper] Token expired, refreshing...");
            } else {
                info!(
                    "[TokenHelper] Token expires in {}s, refreshing proactively...",
                    time_until_expiry
                );
            }

            if let Some(refresh_token) = &token.refresh_token {
                match oauth.refresh_token(refresh_token).await {
                    Ok(_) => {
                        info!("[TokenHelper] Token refreshed successfully!");
                        return true;
                    }
                    Err(e) => {
                        error!("[TokenHelper] Token refresh failed: {}", e);
                        return false;
                    }
                }
            } else {
                warn!("[TokenHelper] No refresh token available");
                return false;
            }
        }

        // Token is still fresh
        return true;
    }

    false // No token available
}

/// Synchronous version - spawns async task and waits
/// Use this from UI thread when you need to ensure valid token before API call
pub fn ensure_fresh_token_sync(oauth: &OAuthManager) -> bool {
    let oauth_clone = oauth.clone();
    let rt = match crate::utils::error_handling::create_runtime() {
        Ok(r) => r,
        Err(e) => {
            log::error!("[TokenHelper] {}", e);
            return false;
        }
    };
    rt.block_on(ensure_fresh_token(&oauth_clone))
}

/// Get valid token synchronously - spawns async task and waits
/// Use this from UI thread before making API calls
pub fn get_valid_token_sync(oauth: &OAuthManager) -> Option<TokenData> {
    let oauth_clone = oauth.clone();
    let rt = match crate::utils::error_handling::create_runtime() {
        Ok(r) => r,
        Err(e) => {
            log::error!("[TokenHelper] {}", e);
            return None;
        }
    };
    rt.block_on(get_valid_token(&oauth_clone))
}
