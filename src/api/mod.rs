// SoundCloud API client modules

pub mod search;
pub mod playlists;
pub mod tracks;
pub mod activities;
pub mod users;
pub mod likes;

// Re-export commonly used functions
pub use search::{search_tracks, search_tracks_smart, search_playlists, search_playlists_paginated};
pub use playlists::{fetch_playlist_by_id, fetch_playlist_chunks};
pub use tracks::{fetch_track_by_id, fetch_related_tracks, load_next_search_page, load_next_search_page_smart};
pub use activities::fetch_recent_activities;
pub use users::{fetch_user_likes, fetch_track_favoriters};
