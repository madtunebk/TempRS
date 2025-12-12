use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;
use rand::RngCore;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

const NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64, // Unix timestamp
    pub token_type: String,
    pub machine_fp: String, // Machine fingerprint for validation
}

/// Derive a 32-byte encryption key from the machine fingerprint
fn derive_key() -> [u8; 32] {
    let fp = crate::utils::fingerprint::fingerprint();
    let mut hasher = Sha256::new();
    hasher.update(fp.as_bytes());
    hasher.update(b"TempRS-token-encryption-v1"); // Salt
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Encrypts a text value before storing it in the database
fn encrypt_text(plain: &str) -> String {
    let key_bytes = derive_key();
    let key = Key::<Aes256Gcm>::from(key_bytes);
    let cipher = Aes256Gcm::new(&key);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plain.as_bytes())
        .expect("encryption should not fail");

    let mut output = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    BASE64_STANDARD.encode(output)
}

/// Decrypts a previously encrypted value
fn decrypt_text(cipher_text: &str) -> Option<String> {
    let raw = BASE64_STANDARD.decode(cipher_text).ok()?;
    if raw.len() <= NONCE_LEN {
        return None;
    }

    let (nonce_bytes, cipher_bytes) = raw.split_at(NONCE_LEN);
    let key_bytes = derive_key();
    let key = Key::<Aes256Gcm>::from(key_bytes);
    let cipher = Aes256Gcm::new(&key);

    let mut nonce_array = [0u8; NONCE_LEN];
    nonce_array.copy_from_slice(nonce_bytes);
    let nonce = Nonce::from(nonce_array);

    cipher
        .decrypt(&nonce, cipher_bytes)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

#[derive(Clone)]
pub struct TokenStore {
    db_path: PathBuf,
}

impl TokenStore {
    pub fn new() -> Self {
        let db_path = Self::get_db_path();

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Initialize database tables with separate encrypted columns
        if let Ok(conn) = Connection::open(&db_path) {
            // Check if old schema exists and migrate
            let has_old_schema = conn
                .prepare("SELECT token_data FROM tokens LIMIT 1")
                .is_ok();

            if has_old_schema {
                log::info!("[TokenStore] Detected old token schema, migrating to new format...");
                // Drop old table
                let _ = conn.execute("DROP TABLE IF EXISTS tokens", []);
                log::info!("[TokenStore] Old tokens cleared, will require re-login");
            }

            // Create new schema
            let _ = conn.execute(
                "CREATE TABLE IF NOT EXISTS tokens (
                    id INTEGER PRIMARY KEY,
                    access_token TEXT NOT NULL,
                    refresh_token TEXT,
                    token_type TEXT NOT NULL,
                    expires_at INTEGER NOT NULL,
                    created_at INTEGER NOT NULL,
                    machine_fp TEXT NOT NULL
                )",
                [],
            );
        }

        Self { db_path }
    }

    fn get_db_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("TempRS");
        path.push("tokens.db");
        path
    }

    fn get_connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
    }

    pub fn save_token(&self, token: &TokenData) -> Result<(), String> {
        let conn = self
            .get_connection()
            .map_err(|e| format!("Failed to open database: {}", e))?;

        // Encrypt each field separately
        let encrypted_access = encrypt_text(&token.access_token);
        let encrypted_refresh = token.refresh_token.as_ref().map(|rt| encrypt_text(rt));
        let encrypted_fp = encrypt_text(&token.machine_fp);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Delete existing tokens (keep only one)
        conn.execute("DELETE FROM tokens", [])
            .map_err(|e| format!("Failed to clear old tokens: {}", e))?;

        conn.execute(
            "INSERT INTO tokens (access_token, refresh_token, token_type, expires_at, created_at, machine_fp) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                encrypted_access,
                encrypted_refresh,
                &token.token_type,
                token.expires_at as i64,
                now,
                encrypted_fp
            ],
        )
        .map_err(|e| format!("Failed to insert token: {}", e))?;

        log::debug!(
            "[TokenStore] Token saved - expires at: {}, created at: {}",
            token.expires_at,
            now
        );
        Ok(())
    }

    pub fn load_token(&self) -> Result<TokenData, String> {
        let conn = self
            .get_connection()
            .map_err(|e| format!("Failed to open database: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT access_token, refresh_token, token_type, expires_at, machine_fp FROM tokens ORDER BY id DESC LIMIT 1")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let token = stmt
            .query_row([], |row| {
                let encrypted_access: String = row.get(0)?;
                let encrypted_refresh: Option<String> = row.get(1)?;
                let token_type: String = row.get(2)?;
                let expires_at: i64 = row.get(3)?;
                let encrypted_fp: String = row.get(4)?;

                Ok((
                    encrypted_access,
                    encrypted_refresh,
                    token_type,
                    expires_at,
                    encrypted_fp,
                ))
            })
            .map_err(|_| "No saved token found".to_string())?;

        let (encrypted_access, encrypted_refresh, token_type, expires_at, encrypted_fp) = token;

        // Decrypt fields
        let access_token = decrypt_text(&encrypted_access)
            .ok_or_else(|| "Failed to decrypt access token".to_string())?;

        let refresh_token = encrypted_refresh.and_then(|enc| decrypt_text(&enc));

        let machine_fp = decrypt_text(&encrypted_fp)
            .ok_or_else(|| "Failed to decrypt machine fingerprint".to_string())?;

        // Validate machine fingerprint
        let current_fp = crate::utils::fingerprint::fingerprint();
        if machine_fp != current_fp {
            return Err("Token is bound to another machine".to_string());
        }

        let token_data = TokenData {
            access_token,
            refresh_token,
            expires_at: expires_at as u64,
            token_type,
            machine_fp,
        };

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        log::debug!(
            "[TokenStore] Token loaded - expires at: {}, current time: {}, valid: {}",
            expires_at,
            current_time,
            expires_at as u64 > current_time
        );

        Ok(token_data)
    }

    #[allow(dead_code)]
    pub fn delete_token(&self) -> Result<(), String> {
        let conn = self
            .get_connection()
            .map_err(|e| format!("Failed to open database: {}", e))?;

        conn.execute("DELETE FROM tokens", [])
            .map_err(|e| format!("Failed to delete token: {}", e))?;

        Ok(())
    }

    pub fn is_token_valid(&self, token: &TokenData) -> bool {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let is_valid = token.expires_at > current_time;

        log::debug!("[TokenStore] Token validity check - expires: {}, current: {}, valid: {}, time_left: {}s",
                 token.expires_at, current_time, is_valid,
                 if is_valid { token.expires_at - current_time } else { 0 });

        is_valid
    }

    pub fn get_valid_token(&self) -> Option<TokenData> {
        match self.load_token() {
            Ok(token) => {
                if self.is_token_valid(&token) {
                    log::debug!("[TokenStore] Returning valid token");
                    Some(token)
                } else {
                    log::debug!("[TokenStore] Token expired, needs refresh");
                    None
                }
            }
            Err(e) => {
                log::debug!("[TokenStore] Failed to load token: {}", e);
                None
            }
        }
    }

    /// Get token regardless of expiry - useful for checking if refresh token exists
    pub fn get_token_for_refresh(&self) -> Option<TokenData> {
        match self.load_token() {
            Ok(token) => {
                log::debug!(
                    "[TokenStore] Token loaded for refresh check - has refresh_token: {}",
                    token.refresh_token.is_some()
                );
                Some(token)
            }
            Err(e) => {
                log::debug!("[TokenStore] No token available for refresh: {}", e);
                None
            }
        }
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}
