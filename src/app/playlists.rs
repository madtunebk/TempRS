// DEPRECATED: This module is kept for backwards compatibility
// New code should use `crate::models::*` and `crate::api::*` directly

// Re-export models
#[allow(unused_imports)]
pub use crate::models::{
    ActivitiesResponse, Activity, ActivityOrigin, FavoritersResponse, Playlist, PlaylistDetailed,
    PlaylistSearchResults, PlaylistsResponse, SearchTracksResponse, Track, TracksResponse, User,
};

// Re-export API functions
#[allow(unused_imports)]
pub use crate::api::{
    fetch_playlist_by_id, fetch_playlist_chunks, fetch_recent_activities, fetch_related_tracks,
    fetch_track_by_id, fetch_track_favoriters, fetch_user_likes, load_next_search_page,
    load_next_search_page_smart, search_playlists, search_playlists_paginated, search_tracks,
    search_tracks_smart,
};
