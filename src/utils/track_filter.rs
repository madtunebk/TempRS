/// Track filtering utilities for geo-locked and non-streamable content
use crate::models::Track;

/// Check if a track is available for playback
/// Filters out:
/// - Geo-locked tracks (policy == "BLOCK")
/// - Non-streamable tracks (streamable != true)
/// - Tracks without stream URLs
/// - Tracks with restricted access
pub fn is_track_playable(track: &Track) -> bool {
    // Check if track is streamable
    if !track.streamable.unwrap_or(false) {
        log::debug!("[TrackFilter] ❌ Track '{}' is not streamable", track.title);
        return false;
    }

    // Check if track has a stream URL
    if track.stream_url.is_none() {
        log::debug!("[TrackFilter] ❌ Track '{}' has no stream URL", track.title);
        return false;
    }

    // Check for geo-lock policy (BLOCK means geo-restricted)
    if let Some(policy) = &track.policy {
        let policy_upper = policy.to_uppercase();
        if policy_upper == "BLOCK" {
            log::debug!(
                "[TrackFilter] 🌍 Track '{}' is geo-locked (policy: {})",
                track.title,
                policy
            );
            return false;
        }
    }

    // Check access restrictions (if "access" is "blocked" or "preview")
    if let Some(access) = &track.access {
        let access_lower = access.to_lowercase();
        if access_lower == "blocked" || access_lower == "preview" {
            log::debug!(
                "[TrackFilter] 🔒 Track '{}' has restricted access: {}",
                track.title,
                access
            );
            return false;
        }
    }

    // Track is playable
    true
}

/// Check if a track is a valid selectable candidate for the queue.
/// Less strict than `is_track_playable`: allows database tracks that are streamable
/// but missing `stream_url` (they will be fetched on-demand when played).
pub fn is_track_selectable(track: &Track) -> bool {
    // Must be marked streamable
    if !track.streamable.unwrap_or(false) {
        return false;
    }
    // Geo-lock policy
    if let Some(policy) = &track.policy {
        if policy.to_uppercase() == "BLOCK" {
            return false;
        }
    }
    // Access restrictions
    if let Some(access) = &track.access {
        let a = access.to_lowercase();
        if a == "blocked" || a == "preview" {
            return false;
        }
    }
    true
}

/// Filter a list of tracks to only include playable ones
pub fn filter_playable_tracks(tracks: Vec<Track>) -> Vec<Track> {
    let original_count = tracks.len();
    let filtered: Vec<Track> = tracks.into_iter().filter(is_track_playable).collect();

    let removed = original_count - filtered.len();
    if removed > 0 {
        log::debug!(
            "[TrackFilter] Filtered out {} non-playable tracks ({} remaining)",
            removed,
            filtered.len()
        );
    }

    filtered
}

/// Remove duplicate tracks by ID, keeping first occurrence
pub fn remove_duplicates(tracks: Vec<Track>) -> Vec<Track> {
    let original_count = tracks.len();
    let mut seen_ids = std::collections::HashSet::new();
    let mut unique_tracks = Vec::new();

    for track in tracks {
        if seen_ids.insert(track.id) {
            unique_tracks.push(track);
        }
    }

    let removed = original_count - unique_tracks.len();
    if removed > 0 {
        log::debug!(
            "[TrackFilter] Removed {} duplicate tracks ({} unique remaining)",
            removed,
            unique_tracks.len()
        );
    }

    unique_tracks
}

/// Filter playable tracks AND remove duplicates
pub fn filter_and_deduplicate(tracks: Vec<Track>) -> Vec<Track> {
    let playable = filter_playable_tracks(tracks);
    remove_duplicates(playable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Track, User};

    fn mock_track(
        streamable: Option<bool>,
        stream_url: Option<String>,
        policy: Option<String>,
        access: Option<String>,
    ) -> Track {
        Track {
            id: 123,
            title: "Test Track".to_string(),
            user: User {
                id: 1,
                username: "Test User".to_string(),
                avatar_url: None,
            },
            streamable,
            stream_url,
            policy,
            access,
            artwork_url: None,
            duration: 180000,
            full_duration: None,
            genre: Some("Electronic".to_string()),
            permalink_url: Some("https://soundcloud.com/test".to_string()),
            playback_count: Some(1000),
        }
    }

    #[test]
    fn test_playable_track() {
        let track = mock_track(
            Some(true),
            Some("https://stream.url".to_string()),
            None,
            None,
        );
        assert!(is_track_playable(&track));
    }

    #[test]
    fn test_non_streamable() {
        let track = mock_track(
            Some(false),
            Some("https://stream.url".to_string()),
            None,
            None,
        );
        assert!(!is_track_playable(&track));
    }

    #[test]
    fn test_streamable_none() {
        let track = mock_track(None, Some("https://stream.url".to_string()), None, None);
        assert!(!is_track_playable(&track));
    }

    #[test]
    fn test_no_stream_url() {
        let track = mock_track(Some(true), None, None, None);
        assert!(!is_track_playable(&track));
    }

    #[test]
    fn test_geo_locked() {
        let track = mock_track(
            Some(true),
            Some("https://stream.url".to_string()),
            Some("BLOCK".to_string()),
            None,
        );
        assert!(!is_track_playable(&track));
    }

    #[test]
    fn test_blocked_access() {
        let track = mock_track(
            Some(true),
            Some("https://stream.url".to_string()),
            None,
            Some("blocked".to_string()),
        );
        assert!(!is_track_playable(&track));
    }

    #[test]
    fn test_preview_access() {
        let track = mock_track(
            Some(true),
            Some("https://stream.url".to_string()),
            None,
            Some("preview".to_string()),
        );
        assert!(!is_track_playable(&track));
    }
}
