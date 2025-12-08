use crate::models::{track::Track, playlist::Playlist};

/// Fetch user's liked tracks
pub async fn fetch_user_liked_tracks(token: &str) -> Result<Vec<Track>, String> {
    let url = "https://api.soundcloud.com/me/likes/tracks?limit=200&access=playable&linked_partitioning=true";
    
    log::info!("[Likes API] Fetching liked tracks from: {}", url);
    
    let response = crate::utils::http::retry_get_with_auth(url, token)
        .await
        .map_err(|e| {
            log::error!("[Likes API] Request failed: {}", e);
            format!("Failed to fetch liked tracks: {}", e)
        })?;
    
    let status = response.status();
    log::info!("[Likes API] Response status: {}", status);
    
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    log::debug!("[Likes API] Response body (first 1000 chars): {}", &body[..body.len().min(1000)]);
    
    let response: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| {
            log::error!("[Likes API] JSON parse error: {}", e);
            format!("Failed to parse liked tracks: {}", e)
        })?;
    
    // Log the full response structure
    if let Some(obj) = response.as_object() {
        log::info!("[Likes API] Response keys: {:?}", obj.keys().collect::<Vec<_>>());
    }
    
    let collection = response["collection"]
        .as_array()
        .ok_or_else(|| {
            log::error!("[Likes API] No 'collection' field in response. Full response: {}", response);
            "No collection in response".to_string()
        })?;
    
    log::info!("[Likes API] Collection has {} items", collection.len());
    
    let tracks: Vec<Track> = collection
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            // Check if item has a "track" field (nested structure)
            let track_data = if item.get("track").is_some() {
                log::debug!("[Likes API] Item {} has nested 'track' field", idx);
                &item["track"]
            } else {
                log::debug!("[Likes API] Item {} is direct track object", idx);
                item
            };
            
            match serde_json::from_value::<Track>(track_data.clone()) {
                Ok(track) => Some(track),
                Err(e) => {
                    log::warn!("[Likes API] Failed to parse track at index {}: {}", idx, e);
                    None
                }
            }
        })
        .collect();
    
    log::info!("[Likes API] Successfully parsed {} tracks out of {} items", tracks.len(), collection.len());
    
    Ok(tracks)
}

/// Fetch user's playlists (created + liked)
pub async fn fetch_user_playlists(token: &str) -> Result<(Vec<Playlist>, Vec<u64>), String> {
    let mut all_playlists = Vec::new();
    let mut created_playlist_ids = Vec::new();
    
    // 1. Fetch user's created playlists
    let created_url = "https://api.soundcloud.com/me/playlists?show_tracks=true&linked_partitioning=true&limit=200";
    log::info!("[Likes API] Fetching created playlists from: {}", created_url);
    
    let response = crate::utils::http::retry_get_with_auth(created_url, token)
        .await
        .map_err(|e| format!("Failed to fetch created playlists: {}", e))?;
    
    let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
    let response: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse created playlists: {}", e))?;
    
    if let Some(collection) = response["collection"].as_array() {
        log::info!("[Likes API] Created playlists collection has {} items", collection.len());
        
        for (idx, item) in collection.iter().enumerate() {
            let playlist_data = if item.get("playlist").is_some() {
                &item["playlist"]
            } else {
                item
            };
            
            match serde_json::from_value::<Playlist>(playlist_data.clone()) {
                Ok(playlist) => {
                    log::debug!("[Likes API] Added created playlist '{}'", playlist.title);
                    created_playlist_ids.push(playlist.id);  // Track as created
                    all_playlists.push(playlist);
                }
                Err(e) => {
                    log::warn!("[Likes API] Failed to parse created playlist at index {}: {}", idx, e);
                }
            }
        }
    }
    
    // 2. Fetch user's liked playlists
    let liked_url = "https://api.soundcloud.com/me/likes/playlists?limit=200&linked_partitioning=true";
    log::info!("[Likes API] Fetching liked playlists from: {}", liked_url);
    
    let response = crate::utils::http::retry_get_with_auth(liked_url, token)
        .await
        .map_err(|e| format!("Failed to fetch liked playlists: {}", e))?;
    
    let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
    let response: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse liked playlists: {}", e))?;
    
    if let Some(collection) = response["collection"].as_array() {
        log::info!("[Likes API] Liked playlists collection has {} items", collection.len());
        
        for (idx, item) in collection.iter().enumerate() {
            let playlist_data = if item.get("playlist").is_some() {
                &item["playlist"]
            } else {
                item
            };
            
            match serde_json::from_value::<Playlist>(playlist_data.clone()) {
                Ok(playlist) => {
                    log::debug!("[Likes API] Added liked playlist '{}'", playlist.title);
                    all_playlists.push(playlist);
                }
                Err(e) => {
                    log::warn!("[Likes API] Failed to parse liked playlist at index {}: {}", idx, e);
                }
            }
        }
    }
    
    log::info!("[Likes API] Total playlists (created + liked): {} (created: {})", all_playlists.len(), created_playlist_ids.len());
    
    Ok((all_playlists, created_playlist_ids))
}

/// Fetch user's liked playlists (separate from created playlists)
#[allow(dead_code)]
pub async fn fetch_user_liked_playlists(token: &str) -> Result<Vec<Playlist>, String> {
    let url = "https://api.soundcloud.com/me/likes/playlists?limit=200&linked_partitioning=true";
    
    log::info!("[Likes API] Fetching liked playlists from: {}", url);
    
    let response = crate::utils::http::retry_get_with_auth(url, token)
        .await
        .map_err(|e| {
            log::error!("[Likes API] Liked playlists request failed: {}", e);
            format!("Failed to fetch liked playlists: {}", e)
        })?;
    
    let body = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    let response: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse liked playlists: {}", e))?;
    
    let playlists = response["collection"]
        .as_array()
        .ok_or("No collection in response")?
        .iter()
        .filter_map(|item| {
            let playlist_obj = &item["playlist"];
            serde_json::from_value(playlist_obj.clone()).ok()
        })
        .collect();
    
    Ok(playlists)
}

/// Fetch user's own uploaded tracks
#[allow(dead_code)]
pub async fn fetch_user_tracks(token: &str) -> Result<Vec<Track>, String> {
    let url = "https://api.soundcloud.com/me/tracks?limit=200&linked_partitioning=true";
    
    log::info!("[Likes API] Fetching user tracks from: {}", url);
    
    let response = crate::utils::http::retry_get_with_auth(url, token)
        .await
        .map_err(|e| format!("Failed to fetch user tracks: {}", e))?;
    
    let body = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    let response: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse user tracks: {}", e))?;
    
    let tracks = response["collection"]
        .as_array()
        .ok_or("No collection in response")?
        .iter()
        .filter_map(|item| {
            serde_json::from_value(item.clone()).ok()
        })
        .collect();
    
    Ok(tracks)
}

/// Like a track
pub async fn like_track(token: &str, track_id: u64) -> Result<(), String> {
    // SoundCloud uses URN format: soundcloud:tracks:{id}
    let track_urn = format!("soundcloud:tracks:{}", track_id);
    let encoded_urn = urlencoding::encode(&track_urn);
    let url = format!("https://api.soundcloud.com/likes/tracks/{}", encoded_urn);
    
    log::info!("[Likes API] Liking track: {} (URN: {})", track_id, track_urn);
    
    let response = crate::utils::http::retry_post_with_auth(&url, token)
        .await
        .map_err(|e| {
            log::error!("[Likes API] Like request failed: {}", e);
            format!("Failed to like track: {}", e)
        })?;
    
    let status = response.status();
    log::info!("[Likes API] Like response status: {}", status);
    
    if status.is_success() {
        Ok(())
    } else {
        let body = response.text().await.unwrap_or_default();
        log::error!("[Likes API] Like failed with body: {}", body);
        Err(format!("Failed to like track: HTTP {}", status))
    }
}

/// Unlike a track
pub async fn unlike_track(token: &str, track_id: u64) -> Result<(), String> {
    // SoundCloud uses URN format: soundcloud:tracks:{id}
    let track_urn = format!("soundcloud:tracks:{}", track_id);
    let encoded_urn = urlencoding::encode(&track_urn);
    let url = format!("https://api.soundcloud.com/likes/tracks/{}", encoded_urn);
    
    log::info!("[Likes API] Unliking track: {} (URN: {})", track_id, track_urn);
    
    let response = crate::utils::http::retry_delete_with_auth(&url, token)
        .await
        .map_err(|e| {
            log::error!("[Likes API] Unlike request failed: {}", e);
            format!("Failed to unlike track: {}", e)
        })?;
    
    let status = response.status();
    log::info!("[Likes API] Unlike response status: {}", status);
    
    if status.is_success() {
        Ok(())
    } else {
        let body = response.text().await.unwrap_or_default();
        log::error!("[Likes API] Unlike failed with body: {}", body);
        Err(format!("Failed to unlike track: HTTP {}", status))
    }
}

/// Like a playlist
pub async fn like_playlist(token: &str, playlist_id: u64) -> Result<(), String> {
    // SoundCloud uses URN format: soundcloud:playlists:{id}
    let playlist_urn = format!("soundcloud:playlists:{}", playlist_id);
    let encoded_urn = urlencoding::encode(&playlist_urn);
    let url = format!("https://api.soundcloud.com/likes/playlists/{}", encoded_urn);
    
    log::info!("[Likes API] Liking playlist: {} (URN: {})", playlist_id, playlist_urn);
    
    let response = crate::utils::http::retry_post_with_auth(&url, token)
        .await
        .map_err(|e| {
            log::error!("[Likes API] Like playlist request failed: {}", e);
            format!("Failed to like playlist: {}", e)
        })?;
    
    let status = response.status();
    log::info!("[Likes API] Like playlist response status: {}", status);
    
    if status.is_success() {
        Ok(())
    } else {
        let body = response.text().await.unwrap_or_default();
        log::error!("[Likes API] Like playlist failed with body: {}", body);
        Err(format!("Failed to like playlist: HTTP {}", status))
    }
}

/// Unlike a playlist
pub async fn unlike_playlist(token: &str, playlist_id: u64) -> Result<(), String> {
    // SoundCloud uses URN format: soundcloud:playlists:{id}
    let playlist_urn = format!("soundcloud:playlists:{}", playlist_id);
    let encoded_urn = urlencoding::encode(&playlist_urn);
    let url = format!("https://api.soundcloud.com/likes/playlists/{}", encoded_urn);
    
    log::info!("[Likes API] Unliking playlist: {} (URN: {})", playlist_id, playlist_urn);
    
    let response = crate::utils::http::retry_delete_with_auth(&url, token)
        .await
        .map_err(|e| {
            log::error!("[Likes API] Unlike playlist request failed: {}", e);
            format!("Failed to unlike playlist: {}", e)
        })?;
    
    let status = response.status();
    log::info!("[Likes API] Unlike playlist response status: {}", status);
    
    if status.is_success() {
        Ok(())
    } else {
        let body = response.text().await.unwrap_or_default();
        log::error!("[Likes API] Unlike playlist failed with body: {}", body);
        Err(format!("Failed to unlike playlist: HTTP {}", status))
    }
}
