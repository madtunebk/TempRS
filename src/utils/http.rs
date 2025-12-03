use once_cell::sync::Lazy;
use reqwest::Client;
use std::time::Duration;

pub static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .pool_max_idle_per_host(8)
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .timeout(Duration::from_secs(30)) // 30 second timeout for API requests
        .build()
        .expect("failed to build reqwest client")
});

// Client that doesn't follow redirects - for getting Location headers
pub static NO_REDIRECT_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .pool_max_idle_per_host(8)
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .timeout(Duration::from_secs(30)) // 30 second timeout
        .build()
        .expect("failed to build no-redirect client")
});

// Client for streaming audio - no timeout, only read timeout per chunk
pub static STREAMING_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .pool_max_idle_per_host(2)
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .timeout(Duration::from_secs(600)) // 10 minute total timeout (for long tracks)
        .read_timeout(Duration::from_secs(30)) // 30s timeout per chunk read
        .build()
        .expect("failed to build streaming client")
});

pub fn client() -> &'static Client {
    &CLIENT
}

pub fn no_redirect_client() -> &'static Client {
    &NO_REDIRECT_CLIENT
}

pub fn streaming_client() -> &'static Client {
    &STREAMING_CLIENT
}

/// Check if an HTTP status code is retryable (transient error)
fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(
        status.as_u16(),
        408 | // Request Timeout
        429 | // Too Many Requests
        500 | // Internal Server Error
        502 | // Bad Gateway
        503 | // Service Unavailable
        504   // Gateway Timeout
    )
}

/// Retry a request with exponential backoff for transient errors
/// Max 3 attempts with delays: 500ms, 1000ms, 2000ms
#[allow(dead_code)]
pub async fn retry_request<F, Fut, T, E>(mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    const MAX_RETRIES: u32 = 3;
    const BASE_DELAY_MS: u64 = 500;
    
    let mut last_error = None;
    
    for attempt in 0..MAX_RETRIES {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                // Check if error message indicates a retryable status
                let error_str = e.to_string();
                let should_retry = error_str.contains("504") 
                    || error_str.contains("503")
                    || error_str.contains("502")
                    || error_str.contains("500")
                    || error_str.contains("429")
                    || error_str.contains("timeout")
                    || error_str.contains("Timeout");
                
                if !should_retry || attempt == MAX_RETRIES - 1 {
                    return Err(e);
                }
                
                // Exponential backoff
                let delay_ms = BASE_DELAY_MS * 2_u64.pow(attempt);
                log::warn!("[HTTP Retry] Attempt {}/{} failed: {}. Retrying in {}ms...", 
                    attempt + 1, MAX_RETRIES, error_str, delay_ms);
                
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                last_error = Some(e);
            }
        }
    }
    
    // This shouldn't be reachable, but just in case
    Err(last_error.unwrap())
}

/// Retry a reqwest GET request specifically
#[allow(dead_code)]
pub async fn retry_get(url: &str) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
    const MAX_RETRIES: u32 = 3;
    const BASE_DELAY_MS: u64 = 500;
    
    for attempt in 0..MAX_RETRIES {
        let response = client().get(url).send().await?;
        let status = response.status();
        
        // Check if we should retry
        if is_retryable_status(status) && attempt < MAX_RETRIES - 1 {
            let delay_ms = BASE_DELAY_MS * 2_u64.pow(attempt);
            log::warn!("[HTTP Retry] Status {} from {}. Retrying in {}ms... (attempt {}/{})",
                status, url, delay_ms, attempt + 1, MAX_RETRIES);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            continue;
        }
        
        // Return response (success or final failure)
        return Ok(response);
    }
    
    // Shouldn't reach here
    Err("Max retries exceeded".into())
}

/// Retry a GET request with authorization header
pub async fn retry_get_with_auth(url: &str, token: &str) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
    const MAX_RETRIES: u32 = 3;
    const BASE_DELAY_MS: u64 = 500;
    
    for attempt in 0..MAX_RETRIES {
        let response = client()
            .get(url)
            .header("Authorization", format!("OAuth {}", token))
            .send()
            .await?;
        
        let status = response.status();
        
        // Check if we should retry
        if is_retryable_status(status) && attempt < MAX_RETRIES - 1 {
            let delay_ms = BASE_DELAY_MS * 2_u64.pow(attempt);
            log::warn!("[HTTP Retry] Status {} from {}. Retrying in {}ms... (attempt {}/{})",
                status, url, delay_ms, attempt + 1, MAX_RETRIES);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            continue;
        }
        
        // Return response (success or final failure)
        return Ok(response);
    }
    
    // Shouldn't reach here
    Err("Max retries exceeded".into())
}

/// Retry a POST request with authorization header
pub async fn retry_post_with_auth(url: &str, token: &str) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
    const MAX_RETRIES: u32 = 3;
    const BASE_DELAY_MS: u64 = 500;
    
    for attempt in 0..MAX_RETRIES {
        let response = client()
            .post(url)
            .header("Authorization", format!("OAuth {}", token))
            .send()
            .await?;
        
        let status = response.status();
        
        if is_retryable_status(status) && attempt < MAX_RETRIES - 1 {
            let delay_ms = BASE_DELAY_MS * 2_u64.pow(attempt);
            log::warn!("[HTTP Retry] POST Status {} from {}. Retrying in {}ms... (attempt {}/{})",
                status, url, delay_ms, attempt + 1, MAX_RETRIES);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            continue;
        }
        
        return Ok(response);
    }
    
    Err("Max retries exceeded".into())
}

/// Retry a DELETE request with authorization header
pub async fn retry_delete_with_auth(url: &str, token: &str) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
    const MAX_RETRIES: u32 = 3;
    const BASE_DELAY_MS: u64 = 500;
    
    for attempt in 0..MAX_RETRIES {
        let response = client()
            .delete(url)
            .header("Authorization", format!("OAuth {}", token))
            .send()
            .await?;
        
        let status = response.status();
        
        if is_retryable_status(status) && attempt < MAX_RETRIES - 1 {
            let delay_ms = BASE_DELAY_MS * 2_u64.pow(attempt);
            log::warn!("[HTTP Retry] DELETE Status {} from {}. Retrying in {}ms... (attempt {}/{})",
                status, url, delay_ms, attempt + 1, MAX_RETRIES);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            continue;
        }
        
        return Ok(response);
    }
    
    Err("Max retries exceeded".into())
}
