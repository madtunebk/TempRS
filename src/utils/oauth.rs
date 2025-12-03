use crate::utils::token_store::{TokenData, TokenStore};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;

const SOUNDCLOUD_AUTH_URL: &str = "https://soundcloud.com/connect";
const SOUNDCLOUD_TOKEN_URL: &str = "https://api.soundcloud.com/oauth2/token";

fn generate_pkce_pair() -> (String, String) {
    use sha2::{Digest, Sha256};
    
    // Get machine fingerprint
    let machine_fp = crate::utils::fingerprint::fingerprint();
    
    // PKCE spec: 32 bytes random + machine fingerprint for uniqueness
    let mut random_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut random_bytes);
    
    // Combine random bytes with machine fingerprint
    let combined = format!("{}{}", 
        URL_SAFE_NO_PAD.encode(&random_bytes),
        machine_fp
    );

    // code_verifier = base64URL(combined)
    let code_verifier = URL_SAFE_NO_PAD.encode(combined.as_bytes());

    // code_challenge = base64URL(sha256(verifier))
    let hash = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hash);

    (code_verifier, code_challenge)
}

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl OAuthConfig {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
        }
    }
}

use std::sync::Mutex;

#[derive(Clone)]
pub struct OAuthManager {
    config: Arc<OAuthConfig>,
    token_store: TokenStore,
    code_verifier: Arc<Mutex<Option<String>>>,
    code_challenge: Arc<Mutex<Option<String>>>,
}

impl OAuthManager {
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            config: Arc::new(config),
            token_store: TokenStore::new(),
            code_verifier: Arc::new(Mutex::new(None)),
            code_challenge: Arc::new(Mutex::new(None)),
        }
    }

    /// Generate the authorization URL for SoundCloud OAuth with PKCE
    pub fn get_authorization_url(&self, state: &str) -> String {
        let (code_verifier, code_challenge) = generate_pkce_pair();
        
        // Store code_verifier for later token exchange
        *self.code_verifier.lock().unwrap() = Some(code_verifier);
        *self.code_challenge.lock().unwrap() = Some(code_challenge.clone());
        
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&code_challenge={}&code_challenge_method=S256&scope=non-expiring&state={}",
            SOUNDCLOUD_AUTH_URL,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(&code_challenge),
            urlencoding::encode(state)
        )
    }

    /// Check if we have a valid stored token (not expired)
    #[allow(dead_code)]
    pub fn has_valid_token(&self) -> bool {
        let has_valid = self.token_store.get_valid_token().is_some();
        log::debug!("[OAuth] has_valid_token check: {}", has_valid);
        has_valid
    }

    /// Get the stored token if valid (not expired)
    pub fn get_token(&self) -> Option<TokenData> {
        self.token_store.get_valid_token()
    }
    
    /// Get token even if expired (for refresh attempts)
    pub fn get_token_for_refresh(&self) -> Option<TokenData> {
        self.token_store.get_token_for_refresh()
    }

    /// Exchange authorization code for access token
    pub async fn exchange_code_for_token(&self, code: &str) -> Result<TokenData, String> {
        let client = reqwest::Client::new();
        
        // Get code_verifier from storage
        let code_verifier = self.code_verifier.lock().unwrap()
            .clone()
            .unwrap_or_default();
        
        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("redirect_uri", &self.config.redirect_uri),
            ("code", code),
            ("code_verifier", &code_verifier),
        ];

        let response = client
            .post(SOUNDCLOUD_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to exchange code: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Token exchange failed: {}", error_text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or("Missing access_token in response")?
            .to_string();

        let expires_in = json["expires_in"]
            .as_u64()
            .unwrap_or(3600); // Default to 1 hour

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let token_data = TokenData {
            access_token,
            refresh_token: json["refresh_token"].as_str().map(|s| s.to_string()),
            expires_at: current_time + expires_in,
            token_type: json["token_type"]
                .as_str()
                .unwrap_or("Bearer")
                .to_string(),
            machine_fp: crate::utils::fingerprint::fingerprint(),
        };

        // Save the token
        self.token_store.save_token(&token_data)?;

        Ok(token_data)
    }

    /// Refresh an access token using a refresh token
    #[allow(dead_code)]
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenData, String> {
        let client = reqwest::Client::new();
        
        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("refresh_token", refresh_token),
        ];

        let response = client
            .post(SOUNDCLOUD_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to refresh token: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Token refresh failed: {}", error_text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse refresh response: {}", e))?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or("Missing access_token in response")?
            .to_string();

        let expires_in = json["expires_in"]
            .as_u64()
            .unwrap_or(3600);

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let token_data = TokenData {
            access_token,
            refresh_token: json["refresh_token"].as_str().map(|s| s.to_string()),
            expires_at: current_time + expires_in,
            token_type: json["token_type"]
                .as_str()
                .unwrap_or("Bearer")
                .to_string(),
            machine_fp: crate::utils::fingerprint::fingerprint(),
        };

        // Save the refreshed token
        self.token_store.save_token(&token_data)?;

        Ok(token_data)
    }

    /// Logout by deleting stored token
    #[allow(dead_code)]
    pub fn logout(&self) -> Result<(), String> {
        self.token_store.delete_token()
    }

    /// Start local server to handle OAuth callback
    pub async fn start_oauth_callback_server(&self) -> Result<String, String> {
        use tiny_http::{Server, Response};
        
        let server = Server::http("127.0.0.1:3000")
            .map_err(|e| format!("Failed to start callback server: {}", e))?;

        log::info!("OAuth callback server listening on http://127.0.0.1:3000");

        for request in server.incoming_requests() {
            let url = request.url().to_string();
            
            // Parse the authorization code from the callback URL
            if let Some(code_start) = url.find("code=") {
                let code_end = url[code_start..].find('&').unwrap_or(url.len() - code_start);
                let code = &url[code_start + 5..code_start + code_end];
                
                let html_response = "<html><head><style>body{font-family:sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;background:#1a1a1a;color:#fff}h1{color:#ff5500}</style></head><body><div><h1>✓ Authorization successful!</h1><p>You can close this window and return to the app.</p></div></body></html>";
                
                let response = Response::from_string(html_response)
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap()
                    );
                request.respond(response).ok();
                
                return Ok(code.to_string());
            }
            
            if url.contains("error=") {
                let html_response = "<html><head><style>body{font-family:sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;background:#1a1a1a;color:#fff}h1{color:#ff5500}</style></head><body><div><h1>✕ Authorization failed!</h1><p>Please close this window and try again.</p></div></body></html>";
                
                let response = Response::from_string(html_response)
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap()
                    );
                request.respond(response).ok();
                return Err("User denied authorization".to_string());
            }
        }

        Err("Server stopped without receiving callback".to_string())
    }
}