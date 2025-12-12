use super::User;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct Track {
    pub id: u64,
    pub title: String,
    pub duration: u64,
    pub full_duration: Option<u64>, // Full uncut duration (may differ from duration for long tracks)
    pub stream_url: Option<String>,
    pub permalink_url: Option<String>,
    pub artwork_url: Option<String>,
    pub user: User,
    pub genre: Option<String>,
    pub playback_count: Option<u64>,
    pub streamable: Option<bool>,
    pub access: Option<String>,
    pub policy: Option<String>, // Geo-lock policy: "ALLOW", "MONETIZE", "SNIP", "BLOCK"
}
