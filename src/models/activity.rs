use serde::Deserialize;
use super::Track;

/// Activity item from /me/activities endpoint
#[derive(Debug, Deserialize, Clone)]
pub struct Activity {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub activity_type: String, // "track-repost", "track", "playlist-repost", etc.
    #[allow(dead_code)]
    pub created_at: String,
    #[allow(dead_code)]
    pub origin: Option<ActivityOrigin>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ActivityOrigin {
    #[allow(dead_code)]
    #[serde(flatten)]
    pub track: Option<Track>,
}

#[derive(Debug, Deserialize)]
pub struct ActivitiesResponse {
    #[allow(dead_code)]
    pub collection: Vec<Activity>,
    #[allow(dead_code)]
    pub next_href: Option<String>,
}
