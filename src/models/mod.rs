// Data models for SoundCloud API entities

pub mod track;
pub mod playlist;
pub mod activity;
pub mod user;
pub mod responses;

// Re-export commonly used types
pub use track::Track;
pub use user::User;
pub use playlist::{Playlist, PlaylistDetailed};
pub use activity::{Activity, ActivityOrigin, ActivitiesResponse};
pub use responses::{
    TracksResponse, PlaylistsResponse, PlaylistSearchResults, 
    SearchTracksResponse, FavoritersResponse
};
