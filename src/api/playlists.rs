// Playlist API endpoints
use crate::models::{Playlist, Track, TracksResponse};

#[allow(dead_code)]
pub async fn fetch_playlist_by_id(
    token: &str,
    playlist_id: u64,
) -> Result<Playlist, Box<dyn std::error::Error>> {
    // First, get the playlist with initial tracks
    let url = format!(
        "https://api.soundcloud.com/playlists/{}?representation=full",
        playlist_id
    );

    log::debug!("[Playlists] Fetching full playlist: {}", url);

    let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()).into());
    }

    let mut playlist: Playlist = response.json().await?;

    // If we have fewer tracks than track_count, fetch remaining via pagination
    if (playlist.tracks.len() as u32) < playlist.track_count {
        let tracks_url = format!(
            "https://api.soundcloud.com/playlists/{}/tracks?limit=100&linked_partitioning=true",
            playlist_id
        );

        let mut all_tracks = Vec::new();
        let mut next_url = Some(tracks_url);

        while let Some(url) = next_url {
            let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

            if !response.status().is_success() {
                break;
            }

            let tracks_response: TracksResponse = response.json().await?;
            all_tracks.extend(tracks_response.collection);
            next_url = tracks_response.next_href;
        }

        if !all_tracks.is_empty() {
            playlist.tracks = all_tracks;
        }
    }

    // Filter out non-playable tracks (geo-locked, non-streamable, etc.)
    playlist.tracks = crate::utils::track_filter::filter_playable_tracks(playlist.tracks);

    Ok(playlist)
}

/// Fetch playlist tracks in chunks (200 tracks per chunk)
/// Sends each chunk via channel as it arrives
pub async fn fetch_playlist_chunks(
    token: &str,
    playlist_id: u64,
    tx: std::sync::mpsc::Sender<Vec<Track>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Start fetching tracks
    let tracks_url = format!(
        "https://api.soundcloud.com/playlists/{}/tracks?limit=200&linked_partitioning=true",
        playlist_id
    );

    log::debug!("[Playlists] Fetching playlist chunks from: {}", tracks_url);

    let mut next_url = Some(tracks_url);
    let mut total_fetched = 0;

    while let Some(url) = next_url {
        let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

        if !response.status().is_success() {
            log::warn!(
                "[Playlists] Warning: Failed to fetch chunk page: {}",
                response.status()
            );
            break;
        }

        let tracks_response: TracksResponse = response.json().await?;
        let chunk_size = tracks_response.collection.len();
        total_fetched += chunk_size;

        log::debug!(
            "[Playlists] Fetched chunk of {} tracks (total: {})",
            chunk_size,
            total_fetched
        );

        // Send chunk immediately
        if let Err(e) = tx.send(tracks_response.collection) {
            log::error!("[Playlists] Failed to send chunk: {}", e);
            break;
        }

        next_url = tracks_response.next_href;

        // Small delay between chunks to avoid overwhelming the receiver
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Send empty vec to signal completion
    let _ = tx.send(Vec::new());

    log::info!(
        "[Playlists] Completed fetching {} total tracks",
        total_fetched
    );

    Ok(())
}
