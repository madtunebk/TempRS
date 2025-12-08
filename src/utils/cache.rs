use std::fs;
use std::path::PathBuf;
use sha2::{Sha256, Digest};
use rusqlite::{Connection, params};
use std::time::{SystemTime, UNIX_EPOCH};

// ====================================
// DATABASE TYPES
// ====================================

pub struct CacheDB {
    conn: Connection,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CacheEntry {
    pub url: String,
    pub cache_type: String,
    pub file_hash: String,
    pub cached_at: u64,
    pub file_size: i64,
    pub is_placeholder: i32,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct CacheStats {
    pub artwork_count: u64,
    pub sidebar_count: u64,
    pub placeholder_count: u64,
    pub total_size: u64,
}

// ====================================
// DATABASE IMPLEMENTATION
// ====================================

impl CacheDB {
    /// Initialize the cache database
    pub fn new() -> Result<Self, rusqlite::Error> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("TempRS");
        
        std::fs::create_dir_all(&cache_dir).ok();
        let db_path = cache_dir.join("cache.db");
        
        let conn = Connection::open(db_path)?;
        
        // Create cache metadata table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache_entries (
                url TEXT PRIMARY KEY,
                cache_type TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                cached_at INTEGER NOT NULL,
                file_size INTEGER,
                is_placeholder INTEGER DEFAULT 0
            )",
            [],
        )?;
        
        // Create index for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cache_type ON cache_entries(cache_type)",
            [],
        )?;
        
        Ok(Self { conn })
    }
    
    /// Check if a URL is cached
    pub fn is_cached(&self, url: &str, cache_type: &str) -> bool {
        let result: Result<i64, _> = self.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE url = ?1 AND cache_type = ?2",
            params![url, cache_type],
            |row| row.get(0),
        );
        
        result.unwrap_or(0) > 0
    }
    
    /// Get cache entry metadata
    #[allow(dead_code)]
    pub fn get_entry(&self, url: &str, cache_type: &str) -> Option<CacheEntry> {
        let result = self.conn.query_row(
            "SELECT url, cache_type, file_hash, cached_at, file_size, is_placeholder 
             FROM cache_entries WHERE url = ?1 AND cache_type = ?2",
            params![url, cache_type],
            |row| {
                Ok(CacheEntry {
                    url: row.get(0)?,
                    cache_type: row.get(1)?,
                    file_hash: row.get(2)?,
                    cached_at: row.get(3)?,
                    file_size: row.get(4)?,
                    is_placeholder: row.get(5)?,
                })
            },
        );
        
        result.ok()
    }
    
    /// Add or update a cache entry
    pub fn set_entry(&self, url: &str, cache_type: &str, file_hash: &str, file_size: u64, is_placeholder: bool) -> Result<(), rusqlite::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.conn.execute(
            "INSERT OR REPLACE INTO cache_entries (url, cache_type, file_hash, cached_at, file_size, is_placeholder)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![url, cache_type, file_hash, now, file_size as i64, is_placeholder as i32],
        )?;
        
        Ok(())
    }
    
    /// Remove a cache entry
    #[allow(dead_code)]
    pub fn remove_entry(&self, url: &str, cache_type: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "DELETE FROM cache_entries WHERE url = ?1 AND cache_type = ?2",
            params![url, cache_type],
        )?;
        
        Ok(())
    }
    
    /// Clear all entries of a specific cache type
    pub fn clear_cache_type(&self, cache_type: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "DELETE FROM cache_entries WHERE cache_type = ?1",
            params![cache_type],
        )?;
        
        Ok(())
    }
    
    /// Get all cache entries of a specific type
    #[allow(dead_code)]
    pub fn get_all_by_type(&self, cache_type: &str) -> Vec<CacheEntry> {
        let mut stmt = match self.conn.prepare(
            "SELECT url, cache_type, file_hash, cached_at, file_size, is_placeholder 
             FROM cache_entries WHERE cache_type = ?1"
        ) {
            Ok(stmt) => stmt,
            Err(_) => return vec![],
        };
        
        let entries = match stmt.query_map(params![cache_type], |row| {
            Ok(CacheEntry {
                url: row.get(0)?,
                cache_type: row.get(1)?,
                file_hash: row.get(2)?,
                cached_at: row.get(3)?,
                file_size: row.get(4)?,
                is_placeholder: row.get(5)?,
            })
        }) {
            Ok(entries) => entries,
            Err(_) => return vec![],
        };
        
        entries.filter_map(|e| e.ok()).collect()
    }
    
    /// Clean up old cache entries (older than days)
    pub fn cleanup_old_entries(&self, days: u64) -> Result<usize, rusqlite::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let cutoff = now - (days * 24 * 60 * 60);
        
        self.conn.execute(
            "DELETE FROM cache_entries WHERE cached_at < ?1",
            params![cutoff],
        )
    }
    
    /// Get total cache count
    #[allow(dead_code)]
    pub fn get_cache_count(&self) -> u64 {
        let result: Result<i64, _> = self.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries",
            [],
            |row| row.get(0),
        );
        
        result.unwrap_or(0) as u64
    }
    
    /// Get cache statistics
    #[allow(dead_code)]
    pub fn get_stats(&self) -> CacheStats {
        let artwork_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE cache_type = 'artwork'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        
        let sidebar_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE cache_type = 'sidebar_artwork'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        
        let placeholder_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE is_placeholder = 1",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        
        let total_size: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(file_size), 0) FROM cache_entries",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        
        CacheStats {
            artwork_count: artwork_count as u64,
            sidebar_count: sidebar_count as u64,
            placeholder_count: placeholder_count as u64,
            total_size: total_size as u64,
        }
    }
}

// ====================================
// FILE SYSTEM CACHE UTILITIES
// ====================================

/// Get OS-appropriate cache directory
pub fn get_cache_dir() -> PathBuf {
    let mut cache_dir = dirs::cache_dir().unwrap_or_else(|| {
        // Fallback to temp dir if cache dir not available
        std::env::temp_dir()
    });
    
    cache_dir.push("TempRS");
    
    // Create if doesn't exist
    let _ = fs::create_dir_all(&cache_dir);
    
    cache_dir
}

/// Generate cache key from URL using SHA256
fn cache_key(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Get artwork cache path
pub fn get_artwork_cache_path(url: &str) -> PathBuf {
    let mut path = get_cache_dir();
    path.push("artwork");
    let _ = fs::create_dir_all(&path);
    path.push(format!("{}.jpg", cache_key(url)));
    path
}

/// Save artwork to cache using track ID as permanent key
pub fn save_artwork_cache(track_id: u64, data: &[u8], is_placeholder: bool) -> Result<(), std::io::Error> {
    let key = format!("track:{}", track_id);
    let path = get_artwork_cache_path(&key);
    fs::write(&path, data)?;
    
    // Update database with track ID as key
    if let Ok(db) = CacheDB::new() {
        let file_hash = cache_key(&key);
        let _ = db.set_entry(
            &key,
            "artwork",
            &file_hash,
            data.len() as u64,
            is_placeholder,
        );
    }
    
    Ok(())
}

/// Load artwork from cache using track ID
pub fn load_artwork_cache(track_id: u64) -> Option<Vec<u8>> {
    let key = format!("track:{}", track_id);
    
    // Check database first
    if let Ok(db) = CacheDB::new() {
        if !db.is_cached(&key, "artwork") {
            return None;
        }
    }
    
    let path = get_artwork_cache_path(&key);
    fs::read(path).ok()
}





/// Clear old cache files (older than 7 days)
#[allow(dead_code)]
pub fn cleanup_old_cache() -> Result<(), std::io::Error> {
    let cache_dir = get_cache_dir();
    let seven_days_ago = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() - (7 * 24 * 60 * 60);
    
    for category in &["artwork", "audio"] {
        let mut category_path = cache_dir.clone();
        category_path.push(category);
        
        if !category_path.exists() {
            continue;
        }
        
        for entry in fs::read_dir(&category_path)? {
            let entry = entry?;
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                        if duration.as_secs() < seven_days_ago {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Get cache statistics (size, file count)
#[allow(dead_code)]
pub fn get_cache_stats() -> (usize, u64) {
    let cache_dir = get_cache_dir();
    let mut file_count = 0;
    let mut total_size = 0u64;
    
    for category in &["artwork", "audio"] {
        let mut category_path = cache_dir.clone();
        category_path.push(category);
        
        if !category_path.exists() {
            continue;
        }
        
        if let Ok(entries) = fs::read_dir(&category_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    file_count += 1;
                    total_size += metadata.len();
                }
            }
        }
    }
    
    (file_count, total_size)
}

/// Clear all cache
#[allow(dead_code)]
pub fn clear_all_cache() -> Result<(), std::io::Error> {
    let cache_dir = get_cache_dir();
    
    for category in &["artwork", "audio"] {
        let mut category_path = cache_dir.clone();
        category_path.push(category);
        
        if category_path.exists() {
            fs::remove_dir_all(&category_path)?;
            fs::create_dir_all(&category_path)?;
        }
    }
    
    // Clear database
    if let Ok(db) = CacheDB::new() {
        for category in &["artwork", "audio"] {
            let _ = db.clear_cache_type(category);
        }
    }
    
    Ok(())
}

/// Clean up old cache entries (older than specified days)
#[allow(dead_code)]
pub fn cleanup_old_cache_db(days: u64) -> Result<(), std::io::Error> {
    if let Ok(db) = CacheDB::new() {
        db.cleanup_old_entries(days)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
    }
    Ok(())
}


