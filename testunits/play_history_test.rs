use std::error::Error;
use rusqlite::Connection;
use sha2::{Sha256, Digest};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce
};
use base64::prelude::*;
use base64::Engine;

const NONCE_LEN: usize = 12;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger
    env_logger::init();
    
    println!("=== SoundCloud Play History API Test ===\n");
    
    // Try to get token from database first
    let token = match get_token_from_db() {
        Ok(t) => {
            println!("✓ Token loaded from database");
            t
        }
        Err(e) => {
            println!("Failed to load token from database: {}", e);
            println!("Falling back to environment variable...\n");
            
            std::env::var("SOUNDCLOUD_TOKEN")
                .expect("Set SOUNDCLOUD_TOKEN environment variable with your OAuth access token")
        }
    };
    
    // Try different play history endpoints (SoundCloud has multiple APIs)
    let endpoints = vec![
        "https://api-v2.soundcloud.com/me/play-history/tracks",
        "https://api-v2.soundcloud.com/me/recent-tracks",
    
    ];
    let client = reqwest::Client::new();
    
    for url in endpoints {
        println!("Testing endpoint: {}", url);
        println!("Authorization: OAuth {}...\n", &token[..20.min(token.len())]);
        
        let response = client
            .get(url)
            .header("Authorization", format!("OAuth {}", token))
            .header("Accept", "application/json")
            .send()
            .await?;
    
        let status = response.status();
        println!("Response Status: {}", status);
        println!("Response Headers:");
        for (key, value) in response.headers() {
            println!("  {}: {:?}", key, value);
        }
        println!();
        
        if status.is_success() {
            let body = response.text().await?;
            
            // Pretty print JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                println!("✓ Success! Response Body (formatted):");
                println!("{}", serde_json::to_string_pretty(&json)?);
                
                // Parse as play history
                if let Some(collection) = json.get("collection").and_then(|c| c.as_array()) {
                    println!("\n=== Play History Summary ===");
                    println!("Total tracks: {}", collection.len());
                    
                    for (i, item) in collection.iter().take(5).enumerate() {
                        println!("\nTrack {}:", i + 1);
                        if let Some(track) = item.get("track") {
                            if let Some(title) = track.get("title").and_then(|t| t.as_str()) {
                                println!("  Title: {}", title);
                            }
                            if let Some(user) = track.get("user").and_then(|u| u.get("username")).and_then(|u| u.as_str()) {
                                println!("  Artist: {}", user);
                            }
                            if let Some(duration) = track.get("duration").and_then(|d| d.as_u64()) {
                                println!("  Duration: {}s", duration / 1000);
                            }
                        }
                        if let Some(played_at) = item.get("played_at").and_then(|p| p.as_str()) {
                            println!("  Played at: {}", played_at);
                        }
                    }
                    
                    if collection.len() > 5 {
                        println!("\n... and {} more tracks", collection.len() - 5);
                    }
                    
                    break; // Stop after first successful endpoint
                }
            } else {
                println!("Response Body (raw text):");
                println!("{}", body);
                break;
            }
        } else {
            let body = response.text().await.unwrap_or_default();
            println!("Error Response:");
            println!("{}", body);
            println!("\n--- Trying next endpoint...\n");
        }
    }
    
    Ok(())
}

/// Get machine fingerprint (exactly as fingerprint.rs does it)
#[cfg(target_os = "linux")]
fn get_machine_fingerprint() -> Result<String, Box<dyn Error>> {
    let machine_id = std::fs::read_to_string("/etc/machine-id")
        .or_else(|_| std::fs::read_to_string("/var/lib/dbus/machine-id"))?
        .trim()
        .to_string();
    
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo")?;
    
    let mut vendor = String::new();
    let mut model = String::new();
    
    for line in cpuinfo.lines() {
        if vendor.is_empty() && line.starts_with("vendor_id") {
            vendor = line.to_string();
        }
        if model.is_empty() && line.starts_with("model name") {
            model = line.to_string();
        }
        if !vendor.is_empty() && !model.is_empty() {
            break;
        }
    }
    
    let cpu_sig = format!("{}{}", vendor, model);
    let machine_key = format!("{}{}", machine_id, cpu_sig);
    
    // Hash and encode as base64 (NO PADDING) - same as fingerprint.rs
    let hash = Sha256::digest(machine_key.as_bytes());
    Ok(base64::engine::general_purpose::STANDARD_NO_PAD.encode(hash))
}

#[cfg(target_os = "windows")]
fn get_machine_fingerprint() -> Result<String, Box<dyn Error>> {
    use winreg::RegKey;
    use winreg::enums::*;
    
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let crypto_key = hklm.open_subkey("SOFTWARE\\Microsoft\\Cryptography")?;
    let machine_guid: String = crypto_key.get_value("MachineGuid")?;
    
    let mut cpu_sig = String::new();
    if let Ok(cpu_key) = hklm.open_subkey("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0") {
        if let Ok(vendor) = cpu_key.get_value::<String, _>("VendorIdentifier") {
            cpu_sig.push_str(&vendor);
        }
        if let Ok(name) = cpu_key.get_value::<String, _>("ProcessorNameString") {
            cpu_sig.push_str(&name);
        }
    }
    
    let machine_key = format!("{}{}", machine_guid, cpu_sig);
    
    // Hash and encode as base64 (NO PADDING) - same as fingerprint.rs
    let hash = Sha256::digest(machine_key.as_bytes());
    Ok(base64::engine::general_purpose::STANDARD_NO_PAD.encode(hash))
}

/// Derive encryption key from machine fingerprint (same as token_store.rs)
fn derive_key() -> [u8; 32] {
    let fp = get_machine_fingerprint().expect("Failed to get machine fingerprint");
    let mut hasher = Sha256::new();
    hasher.update(fp.as_bytes());
    hasher.update(b"TempRS-token-encryption-v1"); // Same salt as token_store.rs
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Decrypt text from database (same logic as token_store.rs)
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

/// Load token from database
fn get_token_from_db() -> Result<String, Box<dyn Error>> {
    let mut db_path = dirs::config_dir().ok_or("Could not find config directory")?;
    db_path.push("TempRS");
    db_path.push("tokens.db");
    
    if !db_path.exists() {
        return Err("Token database not found. Run the app and login first.".into());
    }
    
    println!("Reading from: {:?}", db_path);
    
    let conn = Connection::open(&db_path)?;
    
    // Get token from database - machine_fp is ALSO encrypted!
    let (encrypted_access, expires_at, encrypted_fp): (String, i64, String) = conn.query_row(
        "SELECT access_token, expires_at, machine_fp FROM tokens ORDER BY created_at DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    )?;
    
    // Decrypt machine fingerprint first
    let machine_fp = decrypt_text(&encrypted_fp)
        .ok_or("Failed to decrypt machine fingerprint")?;
    
    // Verify machine fingerprint matches current machine
    let current_fingerprint = get_machine_fingerprint()?;
    
    if current_fingerprint != machine_fp {
        return Err("Token is bound to a different machine!".into());
    }
    
    // Decrypt access token using machine fingerprint as key derivation
    let access_token = decrypt_text(&encrypted_access)
        .ok_or("Failed to decrypt access token")?;
    
    // Check expiry
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    
    if expires_at <= now {
        return Err("Token expired. Please login again in the app.".into());
    }
    
    println!("✓ Machine fingerprint verified");
    println!("✓ Token decrypted successfully");
    println!("Token expires in {} seconds", expires_at - now);
    
    Ok(access_token)
}
