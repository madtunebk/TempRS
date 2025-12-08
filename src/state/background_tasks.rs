use std::sync::mpsc::Receiver;
use egui::ColorImage;
use crate::app::playlists::{Playlist, Track};

pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub playlists: Vec<Playlist>,
    pub next_href: Option<String>,
}

pub struct BackgroundTasks {
    // Search Results
    pub search_rx: Option<Receiver<SearchResults>>,

    // Playlist Loading
    pub playlist_rx: Option<Receiver<Playlist>>,
    pub playlist_chunk_rx: Option<Receiver<Vec<Track>>>,

    // Home Screen Content
    pub home_recently_played_rx: Option<Receiver<Vec<Track>>>,
    pub home_recommendations_rx: Option<Receiver<Vec<Track>>>,
    pub track_fetch_rx: Option<Receiver<Result<Vec<Track>, String>>>,

    // Suggestions Screen
    pub suggestions_rx: Option<Receiver<Vec<Track>>>,

    // Likes Screen
    pub likes_tracks_rx: Option<Receiver<Vec<Track>>>,
    pub user_tracks_rx: Option<Receiver<Vec<Track>>>,

    // Playlists Screen
    pub playlists_rx: Option<Receiver<(Vec<Playlist>, Vec<u64>)>>,

    // User Avatar
    pub user_avatar_rx: Option<Receiver<ColorImage>>,

    // Artwork
    pub artwork_rx: Option<Receiver<ColorImage>>,
}

impl Default for BackgroundTasks {
    fn default() -> Self {
        Self {
            search_rx: None,
            playlist_rx: None,
            playlist_chunk_rx: None,
            home_recently_played_rx: None,
            home_recommendations_rx: None,
            track_fetch_rx: None,
            suggestions_rx: None,
            likes_tracks_rx: None,
            user_tracks_rx: None,
            playlists_rx: None,
            user_avatar_rx: None,
            artwork_rx: None,
        }
    }
}

impl BackgroundTasks {
    /// Check if any background task is active
    pub fn has_active_tasks(&self) -> bool {
        self.search_rx.is_some()
            || self.playlist_rx.is_some()
            || self.playlist_chunk_rx.is_some()
            || self.home_recently_played_rx.is_some()
            || self.home_recommendations_rx.is_some()
            || self.track_fetch_rx.is_some()
            || self.suggestions_rx.is_some()
            || self.likes_tracks_rx.is_some()
            || self.user_tracks_rx.is_some()
            || self.playlists_rx.is_some()
            || self.user_avatar_rx.is_some()
            || self.artwork_rx.is_some()
    }

    /// Clear all task receivers (for cleanup)
    pub fn clear_all(&mut self) {
        self.search_rx = None;
        self.playlist_rx = None;
        self.playlist_chunk_rx = None;
        self.home_recently_played_rx = None;
        self.home_recommendations_rx = None;
        self.track_fetch_rx = None;
        self.suggestions_rx = None;
        self.likes_tracks_rx = None;
        self.user_tracks_rx = None;
        self.playlists_rx = None;
        self.user_avatar_rx = None;
        self.artwork_rx = None;
    }
}
