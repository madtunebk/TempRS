// Track API endpoints
use crate::models::{Track, TracksResponse, SearchTracksResponse};

/// Fetch a single track by ID from the API
pub async fn fetch_track_by_id(
    token: &str,
    track_id: u64,
) -> Result<Track, Box<dyn std::error::Error>> {
    // Use standard SoundCloud API endpoint with numeric ID
    let url = format!("https://api.soundcloud.com/tracks/{}", track_id);

    log::debug!("[API] Fetching track: {}", url);

    let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

    let status = response.status();
    log::debug!("[API] Response status: {}", status);

    if !status.is_success() {
        // Handle 403 Forbidden as non-playable (geo-blocked, private, or deleted)
        if status.as_u16() == 403 {
            return Err(format!("Track {} is not available (restricted/private)", track_id).into());
        }
        return Err(format!("API returned status: {}", status).into());
    }

    let track: Track = response.json().await?;
    
    // Validate track is playable
    if !crate::utils::track_filter::is_track_playable(&track) {
        return Err(format!("Track '{}' is not playable", track.title).into());
    }
    
    Ok(track)
}

/// Fetch related tracks based on a track URN or ID
pub async fn fetch_related_tracks(
    token: &str,
    track_urn: &str,
    limit: usize,
) -> Result<Vec<Track>, Box<dyn std::error::Error>> {
    // Extract numeric ID from URN if needed (soundcloud:tracks:123 -> 123)
    let track_id = if track_urn.starts_with("soundcloud:tracks:") {
        track_urn.strip_prefix("soundcloud:tracks:").unwrap_or(track_urn)
    } else {
        track_urn
    };
    
    let url = format!(
        "https://api.soundcloud.com/tracks/{}/related?access=playable&limit={}&offset=0&linked_partitioning=true",
        track_id,
        limit
    );

    log::debug!("[Related] Fetching from: {}", url);

    let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::error!("[Related] API error {}: {}", status, body);
        return Err(format!("API returned status: {}", status).into());
    }

    let tracks_response: TracksResponse = response.json().await?;
    
    // Filter and deduplicate (removes non-playable and duplicate tracks)
    let filtered_tracks = crate::utils::track_filter::filter_and_deduplicate(tracks_response.collection);
    
    log::info!("[Related] Fetched {} related tracks", filtered_tracks.len());
    Ok(filtered_tracks)
}

#[allow(dead_code)]
pub async fn load_next_search_page(
    token: &str,
    next_href: &str,
) -> Result<SearchTracksResponse, Box<dyn std::error::Error>> {
    load_next_search_page_smart(token, next_href, 24).await
}

pub async fn load_next_search_page_smart(
    token: &str,
    next_href_opt: &str,
    min_results: usize,
) -> Result<SearchTracksResponse, Box<dyn std::error::Error>> {
    let mut all_playable_tracks = Vec::new();
    let mut next_url = Some(next_href_opt.to_string());

    while all_playable_tracks.len() < min_results && next_url.is_some() {
        let url = next_url.clone().unwrap();

        let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

        if !response.status().is_success() {
            if all_playable_tracks.is_empty() {
                return Err(format!("API returned status: {}", response.status()).into());
            } else {
                break;
            }
        }

        let search_response: SearchTracksResponse = response.json().await?;

        let playable_from_page =
            crate::utils::track_filter::filter_playable_tracks(search_response.collection);

        all_playable_tracks.extend(playable_from_page);
        next_url = search_response.next_href;
    }

    Ok(SearchTracksResponse {
        collection: all_playable_tracks,
        next_href: next_url,
    })
}

/// Fetch multiple tracks by IDs in parallel (max 10 concurrent)
/// Useful for batch loading history DB tracks or playlists
pub async fn fetch_tracks_batch(
    token: &str,
    track_ids: Vec<u64>,
) -> Vec<Track> {
    use futures_util::stream::{self, StreamExt};

    log::info!("[BatchFetch] Fetching {} tracks in parallel (max 10 concurrent)", track_ids.len());

    let results = stream::iter(track_ids)
        .map(|track_id| {
            let token = token.to_string();
            async move {
                match fetch_track_by_id(&token, track_id).await {
                    Ok(track) => Some(track),
                    Err(e) => {
                        log::warn!("[BatchFetch] Failed to fetch track {}: {}", track_id, e);
                        None
                    }
                }
            }
        })
        .buffer_unordered(10)  // Max 10 concurrent requests
        .collect::<Vec<_>>()
        .await;

    let tracks: Vec<Track> = results.into_iter().flatten().collect();
    log::info!("[BatchFetch] Successfully fetched {} out of {} tracks", tracks.len(), tracks.len());
    tracks
}
