// SoundCloud API client modules

pub mod activities;
pub mod likes;
pub mod playlists;
pub mod search;
pub mod tracks;
pub mod users;

// Re-export commonly used functions
pub use activities::fetch_recent_activities;
pub use playlists::{fetch_playlist_by_id, fetch_playlist_chunks};
pub use search::{
    search_playlists, search_playlists_paginated, search_tracks, search_tracks_smart,
};
pub use tracks::{
    fetch_related_tracks, fetch_track_by_id, load_next_search_page, load_next_search_page_smart,
};
pub use users::{fetch_track_favoriters, fetch_user_likes};
