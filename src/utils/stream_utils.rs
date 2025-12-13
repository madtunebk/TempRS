/// Resolve SoundCloud streaming redirect to the actual CDN URL.
/// Uses the no-redirect client and reads the `Location` header with max 2 attempts.
pub async fn resolve_redirect(api_url: &str, token: &str) -> Result<String, String> {
    use tokio::time::{sleep, Duration as TokioDuration};
    let client = crate::utils::http::no_redirect_client();

    // Try up to 2 times instead of looping for 10 seconds (saves API calls)
    for attempt in 1..=2 {
        match client
            .get(api_url)
            .header("Authorization", format!("OAuth {}", token))
            .send()
            .await
        {
            Ok(resp) => {
                if let Some(loc) = resp.headers().get("location").and_then(|h| h.to_str().ok()) {
                    return Ok(loc.to_string());
                }
            }
            Err(e) => {
                log::warn!("[StreamUtils] Redirect request failed (attempt {}/2): {}", attempt, e);
            }
        }
        if attempt < 2 {
            sleep(TokioDuration::from_millis(500)).await;
        }
    }

    Err("Redirect failed: no Location header after 2 attempts".to_string())
}

/// Prefetch CDN redirect URL for a track (for auto-play optimization)
pub async fn prefetch_stream_url(api_url: &str, token: &str) -> Result<String, String> {
    use tokio::time::{sleep, Duration as TokioDuration};
    let client = crate::utils::http::no_redirect_client();
    for attempt in 1..=2 {
        match client
            .get(api_url)
            .header("Authorization", format!("OAuth {}", token))
            .send()
            .await
        {
            Ok(response) => {
                if let Some(loc) = response
                    .headers()
                    .get("location")
                    .and_then(|h| h.to_str().ok())
                {
                    return Ok(loc.to_string());
                } else {
                    log::warn!("[Prefetch] No Location header on attempt {}/2", attempt);
                }
            }
            Err(e) => {
                log::warn!(
                    "[Prefetch] Redirect request failed on attempt {}/2: {}",
                    attempt,
                    e
                );
            }
        }
        if attempt < 2 {
            sleep(TokioDuration::from_millis(500)).await;
        }
    }
    Err("No Location header in redirect".to_string())
}
