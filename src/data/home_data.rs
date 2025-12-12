use crate::api::fetch_related_tracks;
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

/// Fetch recommendations based on recently played tracks
/// Uses the most recently played track to find related tracks
/// Fallback: If history empty or API fails, fetch popular/trending tracks
/// Always returns exactly 6 tracks
pub fn fetch_recommendations_async(
    token: String,
    recently_played: Vec<Track>,
    tx: Sender<Vec<Track>>,
    limit: usize,
) {
    crate::utils::async_helper::spawn_fire_and_forget(move || {
        Box::pin(async move {
            let mut recommendations: Vec<Track> = Vec::new();

            // Try to use recently played track first
            if let Some(track) = recently_played.first() {
                let track_urn = format!("soundcloud:tracks:{}", track.id);
                log::info!(
                    "[Home] Finding {} related tracks for most recent: {} ({})",
                    limit,
                    track.title,
                    track_urn
                );

                // Get related tracks
                match fetch_related_tracks(&token, &track_urn, limit).await {
                    Ok(tracks) => {
                        if !tracks.is_empty() {
                            log::info!("[Home] Fetched {} related tracks", tracks.len());
                            recommendations.extend(tracks);
                        } else {
                            log::warn!(
                                "[Home] Related tracks returned empty, falling back to search"
                            );
                        }
                    }
                    Err(e) => {
                        log::error!("[Home] Failed to fetch related tracks: {}, falling back", e);
                    }
                }
            } else {
                log::info!("[Home] No recently played tracks, recommendations will be empty");
            }

            // Final deduplication pass (in case related tracks had duplicates)
            recommendations = crate::utils::track_filter::remove_duplicates(recommendations);

            log::info!(
                "[Home] Sending {} recommendations total",
                recommendations.len()
            );
            let _ = tx.send(recommendations);
            Ok(())
        })
    });
}
