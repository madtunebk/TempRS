use crate::utils::mediaplay::AudioPlayer;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub enum AudioCommand {
    Play {
        url: String,
        token: String,
        track_id: u64,
    },
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    Seek(Duration),
}

pub struct AudioController {
    command_tx: Sender<AudioCommand>,
    position: Arc<Mutex<Duration>>,
    duration: Arc<Mutex<Option<Duration>>>,
    is_finished: Arc<Mutex<bool>>,
    #[allow(dead_code)]
    current_url: Arc<Mutex<Option<String>>>,
    #[allow(dead_code)]
    current_token: Arc<Mutex<Option<String>>>,
    #[allow(dead_code)]
    current_volume: Arc<Mutex<f32>>,
}

impl AudioController {
    pub fn new(
        bass_energy: Arc<std::sync::atomic::AtomicU32>,
        mid_energy: Arc<std::sync::atomic::AtomicU32>,
        high_energy: Arc<std::sync::atomic::AtomicU32>,
    ) -> Self {
        let (command_tx, command_rx): (Sender<AudioCommand>, Receiver<AudioCommand>) = channel();
        let position = Arc::new(Mutex::new(Duration::ZERO));
        let duration = Arc::new(Mutex::new(None));
        let is_finished = Arc::new(Mutex::new(false));
        let current_url = Arc::new(Mutex::new(None));
        let current_token = Arc::new(Mutex::new(None));
        let current_volume = Arc::new(Mutex::new(1.0));

        let position_clone = position.clone();
        let duration_clone = duration.clone();
        let is_finished_clone = is_finished.clone();
        let current_url_clone = current_url.clone();
        let current_token_clone = current_token.clone();
        let current_volume_clone = current_volume.clone();

        std::thread::spawn(move || {
            let rt = match crate::utils::error_handling::create_runtime() {
                Ok(r) => r,
                Err(e) => {
                    log::error!("[AudioController] Failed to create runtime for audio thread: {}", e);
                    return; // Exit thread gracefully
                }
            };
            let mut player: Option<AudioPlayer> = None;

            loop {
                // Handle commands
                while let Ok(cmd) = command_rx.try_recv() {
                    match cmd {
                        AudioCommand::Play { url, token, track_id } => {
                            log::debug!("[AudioController] Received Play command for track {}", track_id);

                            // Reset finished flag BEFORE loading new track
                            if let Some(mut lock) = crate::utils::error_handling::safe_lock(&is_finished_clone, "AudioController") {
                                *lock = false;
                            }

                            // Cleanup old player first to free memory
                            if let Some(mut old_player) = player.take() {
                                log::debug!("[AudioController] Stopping previous player");
                                old_player.stop();
                                drop(old_player);
                            }

                            if let Some(mut lock) = crate::utils::error_handling::safe_lock(&current_url_clone, "AudioController") {
                                *lock = Some(url.clone());
                            }
                            if let Some(mut lock) = crate::utils::error_handling::safe_lock(&current_token_clone, "AudioController") {
                                *lock = Some(token.clone());
                            }

                            log::debug!("[AudioController] Starting cached audio playback...");
                            match rt.block_on(AudioPlayer::new_and_play_cached(
                                &url, 
                                &token, 
                                track_id,
                                Arc::clone(&bass_energy),
                                Arc::clone(&mid_energy),
                                Arc::clone(&high_energy),
                            )) {
                                Ok(mut p) => {
                                    log::info!("[AudioController] Audio playback started");
                                    // Apply stored volume to new player
                                    if let Some(lock) = crate::utils::error_handling::safe_lock(&current_volume_clone, "AudioController") {
                                        let vol = *lock;
                                        p.set_volume(vol);
                                        log::debug!("[AudioController] Applied volume: {}", vol);
                                    }
                                    if let Some(mut lock) = crate::utils::error_handling::safe_lock(&duration_clone, "AudioController") {
                                        *lock = p.get_duration();
                                    }
                                    player = Some(p);
                                }
                                Err(e) => {
                                    log::error!("[AudioController] Error loading audio: {}", e);
                                }
                            }
                        }
                        AudioCommand::Pause => {
                            log::debug!("[AudioController] Received Pause command");
                            if let Some(p) = player.as_mut() {
                                p.pause();
                            }
                        }
                        AudioCommand::Resume => {
                            log::debug!("[AudioController] Received Resume command");
                            if let Some(p) = player.as_mut() {
                                p.resume();
                            }
                        }
                        AudioCommand::Stop => {
                            log::debug!("[AudioController] Received Stop command");
                            if let Some(mut p) = player.take() {
                                p.stop();
                                // Explicitly drop to free memory immediately
                                drop(p);
                            }
                            if let Some(mut lock) = crate::utils::error_handling::safe_lock(&position_clone, "AudioController") {
                                *lock = Duration::ZERO;
                            }
                            if let Some(mut lock) = crate::utils::error_handling::safe_lock(&duration_clone, "AudioController") {
                                *lock = None;
                            }
                            if let Some(mut lock) = crate::utils::error_handling::safe_lock(&is_finished_clone, "AudioController") {
                                *lock = true;
                            }
                        }
                        AudioCommand::SetVolume(vol) => {
                            if let Some(mut lock) = crate::utils::error_handling::safe_lock(&current_volume_clone, "AudioController") {
                                *lock = vol;
                            }
                            if let Some(p) = player.as_mut() {
                                p.set_volume(vol);
                            }
                        }
                        AudioCommand::Seek(pos) => {
                            log::debug!("[AudioController] Received Seek command to {:?}", pos);
                            if let Some(p) = player.as_mut() {
                                let url = crate::utils::error_handling::safe_lock(&current_url_clone, "AudioController")
                                    .and_then(|lock| lock.clone());
                                let token = crate::utils::error_handling::safe_lock(&current_token_clone, "AudioController")
                                    .and_then(|lock| lock.clone());
                                if let (Some(u), Some(t)) = (url, token) {
                                    if let Err(e) = rt.block_on(p.seek(
                                        pos,
                                        &u,
                                        &t,
                                        Arc::clone(&bass_energy),
                                        Arc::clone(&mid_energy),
                                        Arc::clone(&high_energy),
                                    )) {
                                        log::error!("[AudioController] Seek error: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }

                // Update position and finished status
                if let Some(p) = player.as_ref() {
                    if let Some(mut lock) = crate::utils::error_handling::safe_lock(&position_clone, "AudioController") {
                        *lock = p.get_position();
                    }
                    if let Some(mut lock) = crate::utils::error_handling::safe_lock(&is_finished_clone, "AudioController") {
                        *lock = p.is_finished();
                    }
                }

                std::thread::sleep(Duration::from_millis(50));
            }
        });

        Self {
            command_tx,
            position,
            duration,
            is_finished,
            current_url,
            current_token,
            current_volume,
        }
    }

    pub fn play(&self, url: String, token: String, track_id: u64) {
        let _ = self.command_tx.send(AudioCommand::Play { url, token, track_id });
    }

    pub fn pause(&self) {
        let _ = self.command_tx.send(AudioCommand::Pause);
    }

    pub fn resume(&self) {
        let _ = self.command_tx.send(AudioCommand::Resume);
    }

    pub fn stop(&self) {
        let _ = self.command_tx.send(AudioCommand::Stop);
    }

    pub fn set_volume(&self, volume: f32) {
        let _ = self.command_tx.send(AudioCommand::SetVolume(volume));
    }

    pub fn seek(&self, position: Duration) {
        let _ = self.command_tx.send(AudioCommand::Seek(position));
    }

    pub fn get_position(&self) -> Duration {
        crate::utils::error_handling::safe_lock(&self.position, "AudioController")
            .map(|lock| *lock)
            .unwrap_or(Duration::ZERO)
    }

    pub fn get_duration(&self) -> Option<Duration> {
        crate::utils::error_handling::safe_lock(&self.duration, "AudioController")
            .and_then(|lock| *lock)
    }

    pub fn is_finished(&self) -> bool {
        crate::utils::error_handling::safe_lock(&self.is_finished, "AudioController")
            .map(|lock| *lock)
            .unwrap_or(true) // Default to finished on error
    }
}
