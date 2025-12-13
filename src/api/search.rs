// Search API endpoints for tracks and playlists
use crate::models::{PlaylistSearchResults, PlaylistsResponse, SearchTracksResponse};

/// Search tracks with smart pagination - fetches until we have enough playable results
/// Returns exactly `min_results` tracks (or fewer if no more available)
pub async fn search_tracks_smart(
    token: &str,
    query: &str,
    min_results: usize,
) -> Result<SearchTracksResponse, Box<dyn std::error::Error>> {
    let initial_url = format!(
        "https://api.soundcloud.com/tracks?q={}&access=playable&limit=18&linked_partitioning=1",
        urlencoding::encode(query)
    );

    let mut all_playable_tracks = Vec::new();
    let mut next_url = Some(initial_url);
    let mut pages_fetched = 0;
    const MAX_PAGES: usize = 5; // Limit to 5 pages to prevent excessive API calls

    // Keep fetching pages until we have enough playable tracks or run out of pages
    while all_playable_tracks.len() < min_results && next_url.is_some() && pages_fetched < MAX_PAGES {
        let url = next_url.clone().unwrap();

        let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

        let status = response.status();
        if !status.is_success() {
            if pages_fetched == 0 {
                return Err(format!("API returned status: {}", status).into());
            } else {
                // We got some results, just return what we have
                break;
            }
        }

        let search_response: SearchTracksResponse = response.json().await?;
        pages_fetched += 1;

        // Filter this page's tracks
        let playable_from_page =
            crate::utils::track_filter::filter_playable_tracks(search_response.collection);

        all_playable_tracks.extend(playable_from_page);
        next_url = search_response.next_href;

        // Stop if we have enough
        if all_playable_tracks.len() >= min_results {
            break;
        }
    }

    Ok(SearchTracksResponse {
        collection: all_playable_tracks,
        next_href: next_url,
    })
}

#[allow(dead_code)]
pub async fn search_tracks(
    token: &str,
    query: &str,
    limit: usize,
) -> Result<SearchTracksResponse, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.soundcloud.com/tracks?q={}&access=playable&limit={}&linked_partitioning=1",
        urlencoding::encode(query),
        limit
    );

    log::debug!("[Search] Searching tracks: {}", url);

    let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

    let status = response.status();
    log::debug!("[Search] Response status: {}", status);

    if !status.is_success() {
        return Err(format!("API returned status: {}", status).into());
    }

    let search_response: SearchTracksResponse = response.json().await?;

    // Filter out non-playable tracks (geo-locked, non-streamable, no stream URL, etc.)
    let filtered_tracks =
        crate::utils::track_filter::filter_playable_tracks(search_response.collection);

    Ok(SearchTracksResponse {
        collection: filtered_tracks,
        next_href: search_response.next_href,
    })
}

#[allow(dead_code)]
pub async fn search_playlists(
    token: &str,
    query: &str,
    limit: usize,
) -> Result<PlaylistSearchResults, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.soundcloud.com/playlists?q={}&show_tracks=true&limit={}&linked_partitioning=true",
        urlencoding::encode(query),
        limit
    );

    log::debug!("[Playlists] Searching: {}", url);

    let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()).into());
    }

    let playlists_response: PlaylistsResponse = response.json().await?;

    Ok(PlaylistSearchResults {
        collection: playlists_response.collection,
        next_href: playlists_response.next_href,
    })
}

#[allow(dead_code)]
pub async fn search_playlists_paginated(
    token: &str,
    query: &str,
    limit: usize,
) -> Result<PlaylistsResponse, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.soundcloud.com/playlists?q={}&access=playable&limit={}&linked_partitioning=1",
        urlencoding::encode(query),
        limit
    );

    let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()).into());
    }

    let playlists_response: PlaylistsResponse = response.json().await?;

    Ok(playlists_response)
}
