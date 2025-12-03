// Activities API endpoint
use crate::models::{Track, ActivitiesResponse};

/// Fetch recent activities (listening history) - /me/activities/tracks
#[allow(dead_code)]
pub async fn fetch_recent_activities(
    token: &str,
    limit: usize,
) -> Result<Vec<Track>, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.soundcloud.com/me/activities/tracks?access=playable&limit={}",
        limit
    );

    log::debug!("[Activities] Fetching from: {}", url);

    let response = crate::utils::http::retry_get_with_auth(&url, token).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::error!("[Activities] API error {}: {}", status, body);
        return Err(format!("API returned status: {}", status).into());
    }

    let activities: ActivitiesResponse = response.json().await?;
    
    // Extract tracks from activities, filtering out non-track items
    let tracks: Vec<Track> = activities
        .collection
        .into_iter()
        .filter_map(|activity| {
            if activity.activity_type.contains("track") {
                activity.origin.and_then(|origin| origin.track)
            } else {
                None
            }
        })
        .collect();

    log::info!("[Activities] Fetched {} recent tracks", tracks.len());
    Ok(tracks)
}
