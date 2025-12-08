use crate::utils::audio_controller::AudioController;
use crate::app::queue::PlaybackQueue;
use crate::app_state::RepeatMode;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::time::Instant;

pub struct AudioState {
    // Controllers
    pub audio_controller: AudioController,
    pub playback_queue: PlaybackQueue,

    // Current Track Info (9 fields)
    pub current_track_id: Option<u64>,
    pub last_track_id: Option<u64>,
    pub current_title: String,
    pub current_artist: String,
    pub current_genre: Option<String>,
    pub current_duration_ms: u64,
    pub current_stream_url: Option<String>,
    pub current_permalink_url: Option<String>,
    pub track_start_time: Option<Instant>,

    // Real-time FFT Analysis (3 fields)
    pub bass_energy: Arc<AtomicU32>,
    pub mid_energy: Arc<AtomicU32>,
    pub high_energy: Arc<AtomicU32>,

    // Playback Control (6 fields)
    pub is_playing: bool,
    pub shuffle_mode: bool,
    pub repeat_mode: RepeatMode,
    pub volume: f32,
    pub muted: bool,
    pub volume_before_mute: f32,
}

impl Default for AudioState {
    fn default() -> Self {
        let bass_energy = Arc::new(AtomicU32::new(0));
        let mid_energy = Arc::new(AtomicU32::new(0));
        let high_energy = Arc::new(AtomicU32::new(0));

        Self {
            audio_controller: AudioController::new(
                Arc::clone(&bass_energy),
                Arc::clone(&mid_energy),
                Arc::clone(&high_energy),
            ),
            playback_queue: PlaybackQueue::new(),
            current_track_id: None,
            last_track_id: None,
            current_title: String::new(),
            current_artist: String::new(),
            current_genre: None,
            current_duration_ms: 0,
            current_stream_url: None,
            current_permalink_url: None,
            track_start_time: None,
            bass_energy,
            mid_energy,
            high_energy,
            is_playing: false,
            shuffle_mode: false,
            repeat_mode: RepeatMode::None,
            volume: 1.0,
            muted: false,
            volume_before_mute: 1.0,
        }
    }
}

impl AudioState {
    /// Get current playback position
    pub fn get_position(&self) -> std::time::Duration {
        self.audio_controller.get_position()
    }

    /// Get track duration
    pub fn get_duration(&self) -> Option<std::time::Duration> {
        self.audio_controller.get_duration()
    }

    /// Check if track has finished
    pub fn is_finished(&self) -> bool {
        self.audio_controller.is_finished()
    }

    /// Reset current track state (called on stop)
    pub fn reset_track(&mut self) {
        self.current_track_id = None;
        self.current_title.clear();
        self.current_artist.clear();
        self.current_genre = None;
        self.current_duration_ms = 0;
        self.current_stream_url = None;
        self.current_permalink_url = None;
        self.track_start_time = None;
        self.is_playing = false;
    }
}
