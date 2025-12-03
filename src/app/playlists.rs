// DEPRECATED: This module is kept for backwards compatibility
// New code should use `crate::models::*` and `crate::api::*` directly

// Re-export models
#[allow(unused_imports)]
pub use crate::models::{
    Track, User, Playlist, PlaylistDetailed,
    Activity, ActivityOrigin, ActivitiesResponse,
    TracksResponse, PlaylistsResponse, PlaylistSearchResults,
    SearchTracksResponse, FavoritersResponse,
};

// Re-export API functions
#[allow(unused_imports)]
pub use crate::api::{
    search_tracks, search_tracks_smart, search_playlists, search_playlists_paginated,
    fetch_playlist_by_id, fetch_playlist_chunks,
    fetch_track_by_id, fetch_related_tracks, load_next_search_page, load_next_search_page_smart,
    fetch_recent_activities,
    fetch_user_likes, fetch_track_favoriters,
};
