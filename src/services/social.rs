/// Social service for managing likes/unlikes of tracks and playlists
///
/// Consolidates duplicate logic from MusicPlayerApp::toggle_like() and toggle_playlist_like()

use std::collections::HashSet;

/// Target for like/unlike operations
#[derive(Debug, Clone, Copy)]
pub enum LikeTarget {
    Track(u64),
    Playlist(u64),
}

impl LikeTarget {
    pub fn id(&self) -> u64 {
        match self {
            LikeTarget::Track(id) | LikeTarget::Playlist(id) => *id,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            LikeTarget::Track(_) => "track",
            LikeTarget::Playlist(_) => "playlist",
        }
    }
}

/// Result of a toggle operation (for UI updates)
#[derive(Debug)]
pub struct ToggleResult {
    pub is_liked: bool,
    pub success_message: String,
    pub error_message: String,
}

/// Toggle like status for a track or playlist
///
/// This function handles:
/// - Checking current like status
/// - Updating local state (HashSet)
/// - Spawning background API call
/// - Generating appropriate toast messages
pub fn toggle_like(
    target: LikeTarget,
    liked_ids: &mut HashSet<u64>,
    token: Option<String>,
) -> ToggleResult {
    let id = target.id();
    let kind = target.kind();
    let is_liked = liked_ids.contains(&id);

    if is_liked {
        // Unlike operation
        log::info!("[Like] Unliking {} {}", kind, id);
        liked_ids.remove(&id);

        // Spawn background task to unlike via API
        if let Some(token) = token {
            spawn_unlike_task(target, token);
        } else {
            log::warn!("[Like] No token available for unlike {}", kind);
        }

        ToggleResult {
            is_liked: false,
            success_message: format!("Removed from Liked {}s", capitalize(kind)),
            error_message: "Not authenticated".to_string(),
        }
    } else {
        // Like operation
        log::info!("[Like] Liking {} {}", kind, id);
        liked_ids.insert(id);

        // Spawn background task to like via API
        if let Some(token) = token {
            spawn_like_task(target, token);
        } else {
            log::warn!("[Like] No token available for like {}", kind);
        }

        ToggleResult {
            is_liked: true,
            success_message: format!("Added to Liked {}s", capitalize(kind)),
            error_message: "Not authenticated".to_string(),
        }
    }
}

/// Spawn background task to like a track or playlist
fn spawn_like_task(target: LikeTarget, token: String) {
    crate::utils::async_helper::spawn_fire_and_forget(move || {
        Box::pin(async move {
            let result = match target {
                LikeTarget::Track(id) => {
                    crate::api::likes::like_track(&token, id).await
                }
                LikeTarget::Playlist(id) => {
                    crate::api::likes::like_playlist(&token, id).await
                }
            };

            match result {
                Ok(_) => {
                    log::info!("[Like] Successfully liked {} {}", target.kind(), target.id());
                    Ok(())
                }
                Err(e) => {
                    log::error!("[Like] Failed to like {} {}: {}", target.kind(), target.id(), e);
                    Err(e)
                }
            }
        })
    });
}

/// Spawn background task to unlike a track or playlist
fn spawn_unlike_task(target: LikeTarget, token: String) {
    crate::utils::async_helper::spawn_fire_and_forget(move || {
        Box::pin(async move {
            let result = match target {
                LikeTarget::Track(id) => {
                    crate::api::likes::unlike_track(&token, id).await
                }
                LikeTarget::Playlist(id) => {
                    crate::api::likes::unlike_playlist(&token, id).await
                }
            };

            match result {
                Ok(_) => {
                    log::info!("[Like] Successfully unliked {} {}", target.kind(), target.id());
                    Ok(())
                }
                Err(e) => {
                    log::error!("[Like] Failed to unlike {} {}: {}", target.kind(), target.id(), e);
                    Err(e)
                }
            }
        })
    });
}

/// Capitalize first letter of a string
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}
