use std::collections::HashSet;
use crate::app::playlists::{Playlist, Track};
use crate::app_state::AppState;
use crate::data::home_data::HomeContent;
use crate::utils::playback_history::PlaybackHistoryDB;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchType {
    Tracks,
    Playlists,
}

/// Sort order for Likes tab
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LikesSortOrder {
    RecentFirst,    // Most recently liked first (default)
    TitleAZ,        // Alphabetical by title
    ArtistAZ,       // Alphabetical by artist
}

/// Sort order for Playlists tab
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaylistsSortOrder {
    RecentFirst,    // Most recently added first (default)
    NameAZ,         // Alphabetical by name
    TrackCount,     // By number of tracks
}

/// Sort order for Suggestions tab
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SuggestionsSortOrder {
    Default,        // As returned by API (default)
    TitleAZ,        // Alphabetical by title
    ArtistAZ,       // Alphabetical by artist
}

pub struct ContentState {
    // Shared App State (volume, shuffle, repeat persistence)
    pub app_state: AppState,

    // Playback History Database
    pub playback_history: PlaybackHistoryDB,

    // Search Screen (10 fields)
    pub search_query: String,
    pub search_type: SearchType,
    pub search_expanded: bool,
    pub search_results_tracks: Vec<Track>,
    pub search_results_playlists: Vec<Playlist>,
    pub search_loading: bool,
    pub search_next_href: Option<String>,
    pub search_has_more: bool,
    pub search_page: usize,
    pub search_page_size: usize,

    // Playlist View (2 fields)
    pub selected_playlist_id: Option<u64>,
    pub playlist_loading_id: Option<u64>,

    // Home Screen Content (3 fields)
    pub home_content: HomeContent,
    pub home_loading: bool,
    pub home_recommendations_loading: bool,

    // Suggestions Screen (7 fields - added filter/sort)
    pub suggestions_tracks: Vec<Track>,
    pub suggestions_page: usize,
    pub suggestions_page_size: usize,
    pub suggestions_loading: bool,
    pub suggestions_initial_fetch_done: bool,
    pub suggestions_search_filter: String,
    pub suggestions_sort_order: SuggestionsSortOrder,

    // Likes Screen (9 fields - added filter/sort)
    pub likes_tracks: Vec<Track>,
    pub user_tracks: Vec<Track>,
    pub likes_page: usize,
    pub likes_page_size: usize,
    pub likes_loading: bool,
    pub likes_initial_fetch_done: bool,
    pub liked_track_ids: HashSet<u64>,
    pub likes_search_filter: String,
    pub likes_sort_order: LikesSortOrder,

    // Playlists Screen (9 fields - added filter/sort)
    pub playlists: Vec<Playlist>,
    pub liked_playlist_ids: HashSet<u64>,
    pub user_created_playlist_ids: HashSet<u64>,
    pub playlists_page: usize,
    pub playlists_page_size: usize,
    pub playlists_loading: bool,
    pub playlists_initial_fetch_done: bool,
    pub playlists_search_filter: String,
    pub playlists_sort_order: PlaylistsSortOrder,

    // History View (5 fields)
    pub history_page: usize,
    pub history_page_size: usize,
    pub history_total_tracks: usize,
    pub history_search_filter: String,
    pub history_sort_order: crate::screens::history::HistorySortOrder,
}

impl Default for ContentState {
    fn default() -> Self {
        Self {
            app_state: AppState::new(),
            playback_history: PlaybackHistoryDB::default(),
            search_query: String::new(),
            search_type: SearchType::Tracks,
            search_expanded: false,
            search_results_tracks: Vec::new(),
            search_results_playlists: Vec::new(),
            search_loading: false,
            search_next_href: None,
            search_has_more: false,
            search_page: 0,
            search_page_size: 50,
            selected_playlist_id: None,
            playlist_loading_id: None,
            home_content: HomeContent::default(),
            home_loading: false,
            home_recommendations_loading: false,
            suggestions_tracks: Vec::new(),
            suggestions_page: 0,
            suggestions_page_size: 50,
            suggestions_loading: false,
            suggestions_initial_fetch_done: false,
            suggestions_search_filter: String::new(),
            suggestions_sort_order: SuggestionsSortOrder::Default,
            likes_tracks: Vec::new(),
            user_tracks: Vec::new(),
            likes_page: 0,
            likes_page_size: 50,
            likes_loading: false,
            likes_initial_fetch_done: false,
            liked_track_ids: HashSet::new(),
            likes_search_filter: String::new(),
            likes_sort_order: LikesSortOrder::RecentFirst,
            playlists: Vec::new(),
            liked_playlist_ids: HashSet::new(),
            user_created_playlist_ids: HashSet::new(),
            playlists_page: 0,
            playlists_page_size: 50,
            playlists_loading: false,
            playlists_initial_fetch_done: false,
            playlists_search_filter: String::new(),
            playlists_sort_order: PlaylistsSortOrder::RecentFirst,
            history_page: 0,
            history_page_size: 50,
            history_total_tracks: 0,
            history_search_filter: String::new(),
            history_sort_order: crate::screens::history::HistorySortOrder::RecentFirst,
        }
    }
}

impl ContentState {
    /// Check if a track is liked
    pub fn is_track_liked(&self, track_id: u64) -> bool {
        self.liked_track_ids.contains(&track_id)
    }

    /// Check if a playlist is liked
    pub fn is_playlist_liked(&self, playlist_id: u64) -> bool {
        self.liked_playlist_ids.contains(&playlist_id)
    }

    /// Clear search results
    pub fn clear_search(&mut self) {
        self.search_results_tracks.clear();
        self.search_results_playlists.clear();
        self.search_next_href = None;
        self.search_has_more = false;
        self.search_page = 0;
    }

    /// Reset all paginated content (for logout)
    pub fn reset_all_content(&mut self) {
        self.clear_search();
        self.likes_tracks.clear();
        self.user_tracks.clear();
        self.liked_track_ids.clear();
        self.playlists.clear();
        self.liked_playlist_ids.clear();
        self.user_created_playlist_ids.clear();
        self.suggestions_tracks.clear();
        self.home_content = HomeContent::default();
        self.likes_initial_fetch_done = false;
        self.playlists_initial_fetch_done = false;
        self.suggestions_initial_fetch_done = false;
    }
}
