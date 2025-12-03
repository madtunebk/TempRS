use serde::Deserialize;
use super::{Track, User};

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Playlist {
    pub id: u64,
    pub title: String,
    pub user: User,
    #[serde(default)]
    pub tracks: Vec<Track>,
    pub track_count: u32,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PlaylistDetailed {
    pub id: u64,
    pub title: String,
    pub tracks: Vec<Track>,
    pub track_count: u32,
    pub artwork_url: Option<String>,
    pub tracks_uri: Option<String>,
}
