// API response wrapper types
use serde::Deserialize;
use super::{Track, Playlist, User};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TracksResponse {
    pub collection: Vec<Track>,
    pub next_href: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistsResponse {
    pub collection: Vec<Playlist>,
    pub next_href: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PlaylistSearchResults {
    pub collection: Vec<Playlist>,
    pub next_href: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SearchTracksResponse {
    pub collection: Vec<Track>,
    pub next_href: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FavoritersResponse {
    pub collection: Vec<User>,
}
