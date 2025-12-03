use crate::app::playlists::Track;
use rand::seq::SliceRandom;

/// Record a single track to playback history database (called when track actually plays)
pub fn record_track_to_history(track: &Track) {
    use crate::utils::playback_history::{PlaybackHistoryDB, PlaybackRecord};
    
    let record = PlaybackRecord {
        track_id: track.id,
        title: track.title.clone(),
        artist: track.user.username.clone(),
        duration: track.duration,
        genre: track.genre.clone(),
        played_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };
    
    // Record in background to avoid blocking UI
    std::thread::spawn(move || {
        if let Ok(db) = PlaybackHistoryDB::new() {
            if let Err(e) = db.record_playback(&record) {
                log::error!("[PlaybackHistory] Failed to record: {}", e);
            }
        }
    });
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PlaybackQueue {
    /// Original playlist order
    pub original_tracks: Vec<Track>,
    /// Current queue (may be shuffled)
    pub current_queue: Vec<usize>,
    /// Current position in queue
    pub current_index: Option<usize>,
    /// Shuffle state
    pub shuffle_enabled: bool,
}

#[allow(dead_code)]
impl PlaybackQueue {
    pub fn new() -> Self {
        Self {
            original_tracks: Vec::new(),
            current_queue: Vec::new(),
            current_index: None,
            shuffle_enabled: false,
        }
    }

    /// Load tracks into the queue
    pub fn load_tracks(&mut self, tracks: Vec<Track>) {
        // Deduplicate by track ID and filter out non-playable tracks
        let mut seen_ids = std::collections::HashSet::new();
        let deduplicated: Vec<Track> = tracks.into_iter()
            .filter(|t| {
                // Check if playable first
                if !crate::utils::track_filter::is_track_playable(t) {
                    log::warn!("[Queue] Filtering out non-playable track: {} (ID: {})", t.title, t.id);
                    return false;
                }
                // Then deduplicate
                seen_ids.insert(t.id)
            })
            .collect();
        
        log::info!("[Queue] Loaded {} playable tracks (filtered non-playable)", deduplicated.len());
        
        // Note: Tracks are recorded to history individually when actually played
        // not when loaded into queue
        
        self.original_tracks = deduplicated;
        self.rebuild_queue();
    }
    
    /// Append tracks to existing queue (for progressive loading)
    pub fn append_tracks(&mut self, tracks: Vec<Track>) {
        // Build set of existing track IDs
        let existing_ids: std::collections::HashSet<u64> = self.original_tracks.iter()
            .map(|t| t.id)
            .collect();
        
        // Filter out duplicates and non-playable tracks
        let new_tracks: Vec<Track> = tracks.into_iter()
            .filter(|t| {
                // Check if already exists
                if existing_ids.contains(&t.id) {
                    return false;
                }
                // Check if playable
                if !crate::utils::track_filter::is_track_playable(t) {
                    log::warn!("[Queue] Filtering out non-playable track: {} (ID: {})", t.title, t.id);
                    return false;
                }
                true
            })
            .collect();
        
        if new_tracks.is_empty() {
            return; // No new tracks to add
        }
        
        log::info!("[Queue] Appending {} new playable tracks", new_tracks.len());
        
        // Note: Tracks are recorded to history individually when actually played
        // not when loaded into queue
        
        let old_len = self.original_tracks.len();
        self.original_tracks.extend(new_tracks);
        let new_len = self.original_tracks.len();
        
        // Add new indices to queue
        if self.shuffle_enabled {
            // Add new shuffled indices
            let mut new_indices: Vec<usize> = (old_len..new_len).collect();
            new_indices.shuffle(&mut rand::rng());
            self.current_queue.extend(new_indices);
        } else {
            // Add sequential indices
            self.current_queue.extend(old_len..new_len);
        }
        
        // Set current index if this is the first batch
        if self.current_index.is_none() && !self.current_queue.is_empty() {
            self.current_index = Some(0);
        }
    }

    /// Rebuild the queue based on current shuffle state
    pub fn rebuild_queue(&mut self) {
        let len = self.original_tracks.len();
        if len == 0 {
            self.current_queue.clear();
            self.current_index = None;
            return;
        }

        if self.shuffle_enabled {
            // Create shuffled indices
            let mut indices: Vec<usize> = (0..len).collect();
            indices.shuffle(&mut rand::rng());
            self.current_queue = indices;
        } else {
            // Sequential order
            self.current_queue = (0..len).collect();
        }

        // Set current index to 0 (always start from first track)
        if !self.current_queue.is_empty() {
            self.current_index = Some(0);
        } else {
            self.current_index = None;
        }
    }

    /// Get the current track
    pub fn current_track(&self) -> Option<&Track> {
        let queue_idx = self.current_index?;
        let track_idx = self.current_queue.get(queue_idx)?;
        self.original_tracks.get(*track_idx)
    }

    /// Get next track index (doesn't advance)
    pub fn peek_next(&self) -> Option<&Track> {
        let current = self.current_index?;
        if current + 1 < self.current_queue.len() {
            let track_idx = self.current_queue[current + 1];
            self.original_tracks.get(track_idx)
        } else {
            None
        }
    }

    /// Advance to next track
    pub fn next(&mut self) -> Option<&Track> {
        let current = self.current_index?;
        if current + 1 < self.current_queue.len() {
            self.current_index = Some(current + 1);
            self.current_track()
        } else {
            None
        }
    }

    /// Go to previous track
    pub fn previous(&mut self) -> Option<&Track> {
        let current = self.current_index?;
        if current > 0 {
            self.current_index = Some(current - 1);
            self.current_track()
        } else {
            None
        }
    }

    /// Jump to specific track by ID
    pub fn jump_to_track_id(&mut self, track_id: u64) -> Option<&Track> {
        // Find the track in original list
        let original_idx = self.original_tracks.iter().position(|t| t.id == track_id)?;
        
        // Find position in current queue
        let queue_idx = self.current_queue.iter().position(|&idx| idx == original_idx)?;
        
        self.current_index = Some(queue_idx);
        self.current_track()
    }

    /// Jump to specific index in queue
    pub fn jump_to_index(&mut self, queue_index: usize) -> Option<&Track> {
        if queue_index < self.current_queue.len() {
            self.current_index = Some(queue_index);
            self.current_track()
        } else {
            None
        }
    }

    /// Loop back to start
    pub fn loop_to_start(&mut self) -> Option<&Track> {
        if !self.current_queue.is_empty() {
            self.current_index = Some(0);
            self.current_track()
        } else {
            None
        }
    }

    /// Enable/disable shuffle
    pub fn set_shuffle(&mut self, enabled: bool) {
        if self.shuffle_enabled == enabled {
            return;
        }

        self.shuffle_enabled = enabled;
        
        // Save current track
        let current_track_id = self.current_track().map(|t| t.id);
        
        // Rebuild queue
        self.rebuild_queue();
        
        // Restore position to same track
        if let Some(track_id) = current_track_id {
            self.jump_to_track_id(track_id);
        }
    }

    /// Check if at end of queue
    pub fn is_at_end(&self) -> bool {
        if let Some(current) = self.current_index {
            current >= self.current_queue.len().saturating_sub(1)
        } else {
            true
        }
    }

    /// Get current position info
    pub fn position_info(&self) -> (usize, usize) {
        let current = self.current_index.unwrap_or(0);
        let total = self.current_queue.len();
        (current + 1, total) // 1-indexed for display
    }

    /// Get track at original index
    pub fn get_track_at(&self, index: usize) -> Option<&Track> {
        self.original_tracks.get(index)
    }

    /// Get current queue length
    pub fn len(&self) -> usize {
        self.current_queue.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.current_queue.is_empty()
    }
    
    /// Get recent tracks from queue (up to limit)
    /// Returns tracks in reverse chronological order (most recent first)
    pub fn get_recent_tracks(&self, limit: usize) -> Vec<Track> {
        let start_index = self.current_index.unwrap_or(0);
        let mut tracks = Vec::new();
        
        // Get tracks from current position backwards
        for i in (0..=start_index).rev() {
            if tracks.len() >= limit {
                break;
            }
            if let Some(&original_idx) = self.current_queue.get(i) {
                if let Some(track) = self.original_tracks.get(original_idx) {
                    tracks.push(track.clone());
                }
            }
        }
        
        tracks
    }
}
