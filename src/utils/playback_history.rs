/// Playback history database - tracks locally played songs for accurate "Recently Played" section
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackRecord {
    pub track_id: u64,
    pub title: String,
    pub artist: String,
    pub duration: u64,
    pub genre: Option<String>,
    pub played_at: u64, // Unix timestamp
                        // Note: artwork_url and stream_url NOT stored - will be refetched from API when needed
                        // Artwork is cached by track_id anyway, so URL is redundant
}

pub struct PlaybackHistoryDB {
    conn: Connection,
}

impl PlaybackHistoryDB {
    /// Initialize the playback history database
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path();

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(&db_path)?;

        // Migrate: Drop stream_url and artwork_url columns if they exist (URLs expire/change)
        // SQLite doesn't support DROP COLUMN directly, so we recreate the table
        let has_old_columns: Result<i64> = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('playback_history') 
             WHERE name IN ('stream_url', 'artwork_url')",
            [],
            |row| row.get(0),
        );

        if let Ok(count) = has_old_columns {
            if count > 0 {
                log::info!("[PlaybackHistory] Migrating database: removing URL columns (using track_id for lookups)");

                // Create new table without URL columns
                conn.execute(
                    "CREATE TABLE IF NOT EXISTS playback_history_new (
                        track_id INTEGER PRIMARY KEY,
                        title TEXT NOT NULL,
                        artist TEXT NOT NULL,
                        duration INTEGER NOT NULL,
                        genre TEXT,
                        played_at INTEGER NOT NULL
                    )",
                    [],
                )?;

                // Copy data (excluding URL columns)
                conn.execute(
                    "INSERT INTO playback_history_new 
                     SELECT track_id, title, artist, duration, genre, played_at 
                     FROM playback_history",
                    [],
                )?;

                // Drop old table
                conn.execute("DROP TABLE playback_history", [])?;

                // Rename new table
                conn.execute(
                    "ALTER TABLE playback_history_new RENAME TO playback_history",
                    [],
                )?;

                log::info!("[PlaybackHistory] Migration complete!");
            }
        }

        // Create playback history table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS playback_history (
                track_id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                duration INTEGER NOT NULL,
                genre TEXT,
                played_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create index for faster queries by played_at
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_played_at ON playback_history(played_at DESC)",
            [],
        )?;

        Ok(Self { conn })
    }

    fn get_db_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("TempRS");
        path.push("playback_history.db");
        path
    }

    /// Record a track playback (insert or update if already exists)
    pub fn record_playback(&self, record: &PlaybackRecord) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO playback_history 
             (track_id, title, artist, duration, genre, played_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                record.track_id as i64,
                &record.title,
                &record.artist,
                record.duration as i64,
                &record.genre,
                record.played_at as i64,
            ],
        )?;

        log::debug!(
            "[PlaybackHistory] Recorded: {} by {} (ID: {})",
            record.title,
            record.artist,
            record.track_id
        );

        Ok(())
    }

    /// Get recently played tracks (limit: how many to return)
    pub fn get_recent_tracks(&self, limit: usize) -> Vec<PlaybackRecord> {
        let mut stmt = match self.conn.prepare(
            "SELECT track_id, title, artist, duration, genre, played_at 
             FROM playback_history 
             ORDER BY played_at DESC 
             LIMIT ?1",
        ) {
            Ok(stmt) => stmt,
            Err(e) => {
                log::error!("[PlaybackHistory] Failed to prepare query: {}", e);
                return vec![];
            }
        };

        let records = match stmt.query_map(params![limit as i64], |row| {
            Ok(PlaybackRecord {
                track_id: row.get::<_, i64>(0)? as u64,
                title: row.get(1)?,
                artist: row.get(2)?,
                duration: row.get::<_, i64>(3)? as u64,
                genre: row.get(4)?,
                played_at: row.get::<_, i64>(5)? as u64,
            })
        }) {
            Ok(records) => records,
            Err(e) => {
                log::error!("[PlaybackHistory] Failed to query records: {}", e);
                return vec![];
            }
        };

        records.filter_map(|r| r.ok()).collect()
    }

    /// Get recently played tracks with pagination (limit: page size, offset: skip count)
    pub fn get_recent_tracks_paginated(&self, limit: usize, offset: usize) -> Vec<PlaybackRecord> {
        let mut stmt = match self.conn.prepare(
            "SELECT track_id, title, artist, duration, genre, played_at 
             FROM playback_history 
             ORDER BY played_at DESC 
             LIMIT ?1 OFFSET ?2",
        ) {
            Ok(stmt) => stmt,
            Err(e) => {
                log::error!("[PlaybackHistory] Failed to prepare paginated query: {}", e);
                return vec![];
            }
        };

        let records = match stmt.query_map(params![limit as i64, offset as i64], |row| {
            Ok(PlaybackRecord {
                track_id: row.get::<_, i64>(0)? as u64,
                title: row.get(1)?,
                artist: row.get(2)?,
                duration: row.get::<_, i64>(3)? as u64,
                genre: row.get(4)?,
                played_at: row.get::<_, i64>(5)? as u64,
            })
        }) {
            Ok(records) => records,
            Err(e) => {
                log::error!("[PlaybackHistory] Failed to query paginated records: {}", e);
                return vec![];
            }
        };

        records.filter_map(|r| r.ok()).collect()
    }

    /// Get total count of recorded tracks
    #[allow(dead_code)]
    pub fn get_count(&self) -> u64 {
        let result: Result<i64> =
            self.conn
                .query_row("SELECT COUNT(*) FROM playback_history", [], |row| {
                    row.get(0)
                });

        result.unwrap_or(0) as u64
    }

    /// Clear all playback history
    #[allow(dead_code)]
    pub fn clear_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM playback_history", [])?;
        Ok(())
    }

    /// Clean up old records (older than specified days)
    #[allow(dead_code)]
    pub fn cleanup_old_records(&self, days: u64) -> Result<usize> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cutoff = now - (days * 24 * 60 * 60);

        self.conn.execute(
            "DELETE FROM playback_history WHERE played_at < ?1",
            params![cutoff],
        )
    }
}

impl Default for PlaybackHistoryDB {
    fn default() -> Self {
        Self::new().expect("Failed to initialize playback history database")
    }
}
