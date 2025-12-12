use std::time::Instant;

/// Resolve SoundCloud streaming redirect to the actual CDN URL.
/// Uses the no-redirect client and reads the `Location` header with a 10s guard.
pub async fn resolve_redirect(api_url: &str, token: &str) -> Result<String, String> {
    use tokio::time::{sleep, Duration as TokioDuration};
    let client = crate::utils::http::no_redirect_client();
    let start = Instant::now();
    loop {
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
                log::warn!("[StreamUtils] Redirect request failed: {}", e);
            }
        }
        if start.elapsed().as_secs() > 10 {
            return Err("Redirect timeout: no Location header".to_string());
        }
        sleep(TokioDuration::from_millis(200)).await;
    }
}

/// Prefetch CDN redirect URL for a track (for auto-play optimization)
pub async fn prefetch_stream_url(api_url: &str, token: &str) -> Result<String, String> {
    use tokio::time::{sleep, Duration as TokioDuration};
    let client = crate::utils::http::no_redirect_client();
    for attempt in 1..=3 {
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
                    log::warn!("[Prefetch] No Location header on attempt {}/3", attempt);
                }
            }
            Err(e) => {
                log::warn!(
                    "[Prefetch] Redirect request failed on attempt {}/3: {}",
                    attempt,
                    e
                );
            }
        }
        if attempt < 3 {
            sleep(TokioDuration::from_millis(500 * attempt as u64)).await;
        }
    }
    Err("No Location header in redirect".to_string())
}
