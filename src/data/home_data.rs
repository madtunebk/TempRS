/// Home screen data management - handles fetching and caching of personalized content
use crate::models::{Track, User};
use crate::utils::playback_history::PlaybackHistoryDB;
use std::sync::mpsc::Sender;

/// Home screen content sections
#[derive(Debug, Clone)]
pub struct HomeContent {
    pub recently_played: Vec<Track>,
    pub recommendations: Vec<Track>,
    pub initial_fetch_done: bool,
}

impl HomeContent {
    /// Create new empty home content
    pub fn new() -> Self {
        Self {
            recently_played: Vec::new(),
            recommendations: Vec::new(),
            initial_fetch_done: false,
        }
    }

    /// Check if initial fetch is complete (even if empty)
    pub fn has_content(&self) -> bool {
        self.initial_fetch_done
    }

    /// Clear all content
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.recently_played.clear();
        self.recommendations.clear();
    }
}

impl Default for HomeContent {
    fn default() -> Self {
        Self::new()
    }
}

/// Fetch recently played tracks from local database (no API call needed!)
/// Fetches directly from database ordered by played_at DESC for correct chronological order
pub fn fetch_recently_played_async(_token: String, tx: Sender<Vec<Track>>) {
    crate::utils::async_helper::spawn_fire_and_forget(move || {
        Box::pin(async move {
            let mut recent_tracks: Vec<Track> = Vec::new();

            // Fetch tracks from database ordered by played_at DESC (most recent first)
            match PlaybackHistoryDB::new() {
                Ok(db) => {
                    let records = db.get_recent_tracks(6);
                    log::info!(
                        "[Home] Loaded {} tracks from database (ordered by played_at DESC)",
                        records.len()
                    );

                    // Convert PlaybackRecord to Track
                    for record in records {
                        let track = Track {
                            id: record.track_id,
                            title: record.title.clone(),
                            user: User {
                                id: 0,
                                username: record.artist,
                                avatar_url: None,
                            },
                            artwork_url: None, // Will be fetched from API when needed
                            permalink_url: None,
                            duration: record.duration,
                            full_duration: None, // Not stored in history DB
                            genre: record.genre,
                            streamable: Some(true), // Assumed from history, but will be validated
                            stream_url: None,       // Will be fetched fresh from API when needed
                            playback_count: None,
                            access: None,
                            policy: None,
                        };

                        // Note: We can't validate streamability here since we don't have stream_url
                        // The track will be validated when actually played (fetch_and_play_track)
                        recent_tracks.push(track);
                    }
                }
                Err(e) => {
                    log::error!("[Home] Failed to access playback history database: {}", e);
                }
            }

            log::info!(
                "[Home] Sending {} recently played tracks (ordered by played_at DESC)",
                recent_tracks.len()
            );
            let _ = tx.send(recent_tracks);
            Ok(())
        })
    });
}

/// Fetch recommendations based on local playback history
/// NO API CALLS - uses only local database to prevent spam
/// Returns tracks from history that aren't in the recently played section
pub fn fetch_recommendations_async(
    _token: String,
    recently_played: Vec<Track>,
    tx: Sender<Vec<Track>>,
    limit: usize,
) {
    crate::utils::async_helper::spawn_fire_and_forget(move || {
        Box::pin(async move {
            let mut recommendations: Vec<Track> = Vec::new();

            log::info!(
                "[Home] Generating {} recommendations from local history (no API call)",
                limit
            );

            // Fetch more tracks from history than we need (to have enough after filtering)
            match PlaybackHistoryDB::new() {
                Ok(db) => {
                    // Get more records than limit to ensure we have enough after filtering
                    let records = db.get_recent_tracks(limit * 3);

                    // Build set of recently played track IDs to exclude
                    let recently_played_ids: std::collections::HashSet<u64> =
                        recently_played.iter().map(|t| t.id).collect();

                    log::info!(
                        "[Home] Loaded {} tracks from history, filtering out {} recently played",
                        records.len(),
                        recently_played_ids.len()
                    );

                    // Convert PlaybackRecord to Track, excluding recently played
                    for record in records {
                        // Skip if already in recently played section
                        if recently_played_ids.contains(&record.track_id) {
                            continue;
                        }

                        let track = Track {
                            id: record.track_id,
                            title: record.title.clone(),
                            user: User {
                                id: 0,
                                username: record.artist,
                                avatar_url: None,
                            },
                            artwork_url: None, // Will be fetched from API when needed
                            permalink_url: None,
                            duration: record.duration,
                            full_duration: None,
                            genre: record.genre,
                            streamable: Some(true),
                            stream_url: None, // Will be fetched fresh from API when needed
                            playback_count: None,
                            access: None,
                            policy: None,
                        };

                        recommendations.push(track);

                        // Stop once we have enough recommendations
                        if recommendations.len() >= limit {
                            break;
                        }
                    }
                }
                Err(e) => {
                    log::error!("[Home] Failed to access playback history database: {}", e);
                }
            }

            log::info!(
                "[Home] Sending {} recommendations from local history",
                recommendations.len()
            );
            let _ = tx.send(recommendations);
            Ok(())
        })
    });
}
