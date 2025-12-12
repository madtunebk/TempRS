// User API endpoints
use crate::models::{FavoritersResponse, Track, TracksResponse, User};

/// Fetch users who favorited a track (for recommendations)
/// Returns their user info to fetch their liked tracks
#[allow(dead_code)]
pub async fn fetch_track_favoriters(
    token: &str,
    track_urn: &str,
    limit: usize,
) -> Result<Vec<User>, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.soundcloud.com/tracks/{}/favoriters?limit={}&linked_partitioning=true",
        track_urn, limit
    );

    log::debug!("[Favoriters] Fetching from: {}", url);

    let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::error!("[Favoriters] API error {}: {}", status, body);
        return Err(format!("API returned status: {}", status).into());
    }

    let favoriters: FavoritersResponse = response.json().await?;

    log::info!(
        "[Favoriters] Fetched {} favoriters",
        favoriters.collection.len()
    );
    Ok(favoriters.collection)
}

/// Fetch a user's liked tracks (favorites)
#[allow(dead_code)]
pub async fn fetch_user_likes(
    token: &str,
    user_id: u64,
    limit: usize,
) -> Result<Vec<Track>, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.soundcloud.com/users/{}/favorites?limit={}",
        user_id, limit
    );

    log::debug!("[UserLikes] Fetching from: {}", url);

    let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::error!("[UserLikes] API error {}: {}", status, body);
        return Err(format!("API returned status: {}", status).into());
    }

    let tracks_response: TracksResponse = response.json().await?;

    log::info!(
        "[UserLikes] Fetched {} liked tracks",
        tracks_response.collection.len()
    );
    Ok(tracks_response.collection)
}
