// Data models for SoundCloud API entities

pub mod activity;
pub mod playlist;
pub mod responses;
pub mod track;
pub mod user;

// Re-export commonly used types
pub use activity::{ActivitiesResponse, Activity, ActivityOrigin};
pub use playlist::{Playlist, PlaylistDetailed};
pub use responses::{
    FavoritersResponse, PlaylistSearchResults, PlaylistsResponse, SearchTracksResponse,
    TracksResponse,
};
pub use track::Track;
pub use user::User;
