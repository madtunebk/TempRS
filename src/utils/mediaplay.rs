use rodio::{OutputStream, Sink, Source};
use minimp3::{Decoder as Mp3Decoder, Frame};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

#[allow(dead_code)]
const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks for streaming

// Adaptive timeout constants for different streaming phases
const TIMEOUT_INITIAL_BUFFERING: Duration = Duration::from_secs(12);  // Initial load: API + CDN + buffer
const TIMEOUT_MID_PLAYBACK: Duration = Duration::from_secs(5);         // Mid-stream: sample flow only
const TIMEOUT_HISTORY_TRACK: Duration = Duration::from_secs(15);       // DB tracks: extra API fetch time
const MIN_BUFFERING_SAMPLES: usize = 88200;  // 2 seconds @ 44.1kHz (increased from 1s)

pub struct AudioPlayer {
    sink: Sink,
    _stream: OutputStream,
    stream_handle: rodio::OutputStreamHandle,
    total_duration: Option<Duration>,
    start_time: Instant,
    start_position: Duration,
    paused_at: Option<Duration>,
    #[allow(dead_code)]
    current_url: String,
    #[allow(dead_code)]
    current_token: String,
    current_volume: f32,
    is_history_track: bool,  // Track whether this is a history DB track (for adaptive timeout)
    #[allow(dead_code)]
    stream_thread: Option<std::thread::JoinHandle<()>>,
    shutdown_signal: Arc<AtomicBool>,  // Signal streaming thread to stop
}

/// Progressive streaming source that decodes MP3 chunks as they arrive
struct StreamingSource {
    sample_rx: Receiver<Vec<i16>>,
    current_samples: Vec<i16>,
    sample_index: usize,
    sample_rate: u32,
    channels: u16,
    finished: Arc<Mutex<bool>>,
    buffering: bool,  // Track if we're still buffering initial data
    samples_received: usize,  // Count total samples for stuck detection
    last_sample_time: Instant,  // Detect stream timeout
    fft_tx: Option<Sender<Vec<i16>>>,  // Send samples to FFT as they're played
    fft_buffer: Vec<i16>,  // Buffer for sending to FFT

    // Adaptive timeout fields
    is_history_track: bool,           // Track from DB (requires longer timeout)
    initial_buffering_complete: bool, // Separate flag for initial vs mid-stream
    network_quality_factor: f32,      // 1.0 = good, 1.5 = poor (adjusts timeouts)
}

impl StreamingSource {
    fn new(
        sample_rx: Receiver<Vec<i16>>,
        sample_rate: u32,
        channels: u16,
        finished: Arc<Mutex<bool>>,
        fft_tx: Option<Sender<Vec<i16>>>,
        is_history_track: bool,
        network_quality_factor: f32,
    ) -> Self {
        Self {
            sample_rx,
            current_samples: Vec::new(),
            sample_index: 0,
            sample_rate,
            channels,
            finished,
            buffering: true,
            samples_received: 0,
            last_sample_time: Instant::now(),
            fft_tx,
            fft_buffer: Vec::with_capacity(2048),
            is_history_track,
            initial_buffering_complete: false,
            network_quality_factor,
        }
    }
}

impl Iterator for StreamingSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        // Return current sample if available
        if self.sample_index < self.current_samples.len() {
            let sample = self.current_samples[self.sample_index];
            self.sample_index += 1;
            
            // Send to FFT as samples are being played
            if let Some(tx) = &self.fft_tx {
                self.fft_buffer.push(sample);
                // Send in chunks of ~1152 samples (typical MP3 frame size)
                if self.fft_buffer.len() >= 1152 {
                    let _ = tx.send(self.fft_buffer.clone());
                    self.fft_buffer.clear();
                }
            }
            
            return Some(sample);
        }

        // Try to get next chunk
        match self.sample_rx.try_recv() {
            Ok(samples) => {
                self.current_samples = samples;
                self.sample_index = 0;
                self.samples_received += self.current_samples.len();
                self.last_sample_time = Instant::now();

                // Mark as buffered after receiving substantial data
                if self.buffering && self.samples_received > 44100 {  // ~1 second of audio
                    self.buffering = false;
                }

                // Mark initial buffering complete after 2 seconds of audio
                if !self.initial_buffering_complete && self.samples_received > MIN_BUFFERING_SAMPLES {
                    self.initial_buffering_complete = true;
                    log::info!("[StreamingSource] Initial buffering complete ({} samples)", self.samples_received);
                }
                
                if !self.current_samples.is_empty() {
                    let sample = self.current_samples[0];
                    self.sample_index = 1;
                    Some(sample)
                } else {
                    None
                }
            }
            Err(_) => {
                // Adaptive timeout based on playback phase
                let base_timeout = if !self.initial_buffering_complete {
                    // INITIAL BUFFERING: More lenient
                    if self.is_history_track {
                        TIMEOUT_HISTORY_TRACK
                    } else {
                        TIMEOUT_INITIAL_BUFFERING
                    }
                } else {
                    // MID-PLAYBACK: Strict timeout for stuck detection
                    TIMEOUT_MID_PLAYBACK
                };

                // Apply network quality adjustment
                let adjusted_timeout = Duration::from_secs_f32(
                    base_timeout.as_secs_f32() * self.network_quality_factor
                );

                let timeout = self.last_sample_time.elapsed() > adjusted_timeout;
                let is_finished = *self.finished.lock().unwrap();

                if is_finished && self.initial_buffering_complete {
                    // Stream ended cleanly after buffering completed
                    None
                } else if timeout {
                    // Stream stuck - force end to prevent infinite silence
                    log::error!(
                        "[StreamingSource] Stream timeout after {:?} (phase: {}, quality: {:.1}x, timeout: {:?})",
                        self.last_sample_time.elapsed(),
                        if self.initial_buffering_complete { "playback" } else { "buffering" },
                        self.network_quality_factor,
                        adjusted_timeout
                    );
                    *self.finished.lock().unwrap() = true;
                    None
                } else {
                    // Yield silence while waiting for more data
                    Some(0)
                }
            }
        }
    }
}

impl Source for StreamingSource {
    fn current_frame_len(&self) -> Option<usize> {
        None // Unknown for streaming
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None // Unknown for streaming
    }
}

impl AudioPlayer {
    /// Create new player with progressive streaming - no full download!
    pub async fn new_and_play_cached(
        url: &str,
        token: &str,
        track_id: u64,
        duration_ms: u64,
        bass_energy: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        mid_energy: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        high_energy: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        is_history_track: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let fft_enabled = bass_energy.is_some();
        if fft_enabled {
            log::info!("[AudioPlayer] Starting progressive streaming for track {} (FFT enabled)", track_id);
        } else {
            log::info!("[AudioPlayer] Starting progressive streaming for track {} (FFT disabled)", track_id);
        }

        let (_stream, stream_handle) = OutputStream::try_default()?;

        // Create dual channels - one for audio playback, one for FFT (if enabled)
        let (sample_tx, sample_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
        let (fft_download_tx, fft_download_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = if fft_enabled {
            channel()
        } else {
            // Create dummy channels that won't be used
            channel()
        };
        let (fft_playback_tx, fft_playback_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = if fft_enabled {
            channel()
        } else {
            // Create dummy channels that won't be used
            channel()
        };
        
        let finished = Arc::new(Mutex::new(false));
        let finished_clone = Arc::clone(&finished);

        // Create shutdown signal for graceful thread termination
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown_signal);

        let url_owned = url.to_string();
        let token_owned = token.to_string();
        let cache_key = format!("audio_{}", track_id);

        // Spawn streaming thread that sends to audio + FFT download channel
        let stream_thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                // TODO: Pass prefetched CDN URL from AudioState
                if let Err(e) = stream_audio(&url_owned, &token_owned, &cache_key, sample_tx, fft_download_tx, finished_clone, shutdown_clone, None).await {
                    log::error!("[AudioPlayer] Streaming error: {}", e);
                }
            });
        });
        
        // Wait briefly for first chunk to determine sample rate/channels
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        let sample_rate = 44100; // Default for MP3
        let channels = 2; // Stereo default

        // Create FFT analyzer only if FFT is enabled (GPU mode)
        if fft_enabled {
            if let (Some(bass), Some(mid), Some(high)) = (bass_energy.clone(), mid_energy.clone(), high_energy.clone()) {
                let analyzer = crate::utils::audio_analyzer::AudioAnalyzer::new(
                    bass,
                    mid,
                    high,
                );

                let analyzer_arc = Arc::new(Mutex::new(analyzer));

                // Spawn dedicated FFT processing thread (merges download + playback samples)
                let fft_analyzer = Arc::clone(&analyzer_arc);
                std::thread::spawn(move || {
            log::info!("[FFT] Dedicated FFT processing thread started");
            let mut sample_count = 0usize;
            
            // Process samples from both download and playback channels
            loop {
                let mut got_sample = false;
                
                // Try download channel first (during buffering)
                match fft_download_rx.try_recv() {
                    Ok(samples) => {
                        sample_count += samples.len();
                        if let Ok(mut a) = fft_analyzer.lock() {
                            a.process_samples(&samples);
                        }
                        got_sample = true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        // Download channel closed, that's normal
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // No data yet
                    }
                }
                
                // Try playback channel (during playback)
                match fft_playback_rx.try_recv() {
                    Ok(samples) => {
                        sample_count += samples.len();
                        if let Ok(mut a) = fft_analyzer.lock() {
                            a.process_samples(&samples);
                        }
                        got_sample = true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        // Playback channel closed - this means track ended
                        log::info!("[FFT] Playback channel disconnected, ending FFT processing");
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // No data yet
                    }
                }
                
                // If no samples from either channel, sleep briefly
                if !got_sample {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
            
            log::info!("[FFT] FFT processing thread terminated, processed ~{} samples", sample_count);
                });
            }
        } else {
            log::info!("[AudioPlayer] FFT disabled - skipping FFT analyzer initialization");
        }

        // Audio source sends samples to FFT as they're played (continues after download ends)
        // Only pass fft_playback_tx if FFT is enabled
        let source = StreamingSource::new(
            sample_rx,
            sample_rate,
            channels,
            finished,
            if fft_enabled { Some(fft_playback_tx) } else { None },
            is_history_track,
            1.0, // network_quality_factor: 1.0 = good (TODO: pass from AudioState)
        );
        
        let sink = Sink::try_new(&stream_handle)?;
        sink.append(source);
        log::info!("[AudioPlayer] Progressive streaming started - playing as we download!");
        
        // Convert track duration from milliseconds to Duration
        let total_duration = if duration_ms > 0 {
            let dur = Duration::from_millis(duration_ms);
            log::info!("[AudioPlayer] Track duration set: {}ms = {} seconds ({} minutes)", 
                duration_ms, dur.as_secs(), dur.as_secs() / 60);
            Some(dur)
        } else {
            log::warn!("[AudioPlayer] Track has zero/invalid duration: {}ms", duration_ms);
            None
        };
        
        Ok(Self {
            sink,
            _stream,
            stream_handle: stream_handle.clone(),
            total_duration,
            start_time: Instant::now(),
            start_position: Duration::ZERO,
            paused_at: None,
            current_url: url.to_string(),
            current_token: token.to_string(),
            current_volume: 1.0,
            is_history_track,
            stream_thread: Some(stream_thread),
            shutdown_signal,
        })
    }

    pub fn pause(&mut self) {
        if !self.sink.is_paused() {
            self.paused_at = Some(self.get_position());
            self.sink.pause();
            log::debug!("[AudioPlayer] Paused at {:?}", self.paused_at);
        }
    }

    pub fn resume(&mut self) {
        if self.sink.is_paused() {
            if let Some(paused) = self.paused_at {
                self.start_position = paused;
                self.start_time = Instant::now();
                log::debug!("[AudioPlayer] Resuming from {:?}", paused);
            }
            self.sink.play();
            self.paused_at = None;
        }
    }

    pub fn stop(&mut self) {
        log::debug!("[AudioPlayer] Stopping playback and cleaning up streaming thread");

        // Signal streaming thread to stop
        self.shutdown_signal.store(true, Ordering::Relaxed);

        // Stop sink
        self.sink.stop();

        // Spawn detached cleanup thread - doesn't block caller
        if let Some(thread) = self.stream_thread.take() {
            log::debug!("[Cleanup] Spawning detached cleanup for old streaming thread");

            std::thread::spawn(move || {
                log::debug!("[Cleanup] Waiting for old streaming thread to terminate...");
                let start = std::time::Instant::now();
                while !thread.is_finished() && start.elapsed() < Duration::from_secs(2) {
                    std::thread::sleep(Duration::from_millis(50));
                }
                let _ = thread.join();
                log::debug!("[Cleanup] Old streaming thread terminated");
            });
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.current_volume = volume;
        self.sink.set_volume(volume);
    }

    pub fn is_finished(&self) -> bool {
        if self.paused_at.is_some() {
            return false;
        }

        // Check if sink is empty
        if !self.sink.empty() {
            return false;
        }

        // ADDITIONAL CHECK: Verify we're actually near the end
        if let Some(total_duration) = self.total_duration {
            let current_pos = self.get_position();

            // Only consider finished if within last 2 seconds OR past end
            let time_remaining = total_duration.saturating_sub(current_pos);
            if time_remaining > Duration::from_secs(2) {
                log::debug!("[AudioPlayer] Sink empty but {} seconds remaining - not finished",
                    time_remaining.as_secs());
                return false;
            }
        }

        true
    }

    pub fn get_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    pub fn get_position(&self) -> Duration {
        if let Some(paused) = self.paused_at {
            return paused;
        }

        // Calculate position based on wall clock
        let elapsed = self.start_time.elapsed();
        let calculated_pos = self.start_position.saturating_add(elapsed);

        // If sink is empty AND we've reached/passed the end, STOP incrementing
        if self.sink.empty() {
            if let Some(total) = self.total_duration {
                if calculated_pos >= total {
                    log::debug!("[Position] Sink empty and past end - clamping to total duration");
                    return total;
                }
            }
        }

        // Otherwise return calculated position, clamped to total
        if let Some(total) = self.total_duration {
            calculated_pos.min(total)
        } else {
            calculated_pos
        }
    }

    pub async fn seek(
        &mut self,
        position: Duration,
        url: &str,
        token: &str,
        bass_energy: Option<Arc<std::sync::atomic::AtomicU32>>,
        mid_energy: Option<Arc<std::sync::atomic::AtomicU32>>,
        high_energy: Option<Arc<std::sync::atomic::AtomicU32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("[AudioPlayer] Seeking to {:?}, stopping old stream...", position);

        // STEP 1: Signal old streaming thread to stop
        self.shutdown_signal.store(true, Ordering::Relaxed);

        // STEP 2: Stop current playback
        self.sink.stop();

        // STEP 3: Spawn detached cleanup thread - doesn't block caller
        if let Some(thread) = self.stream_thread.take() {
            log::debug!("[Cleanup] Spawning detached cleanup for old streaming thread");

            std::thread::spawn(move || {
                log::debug!("[Cleanup] Waiting for old streaming thread to terminate...");
                let start = std::time::Instant::now();
                while !thread.is_finished() && start.elapsed() < Duration::from_secs(2) {
                    std::thread::sleep(Duration::from_millis(50));
                }
                let _ = thread.join();
                log::debug!("[Cleanup] Old streaming thread terminated");
            });
        }

        // Estimate byte offset (MP3 is typically 128kbps = 16KB/s)
        let bytes_per_second = 16_000;
        let byte_offset = position.as_secs() * bytes_per_second;

        // Retry seek up to 3 times (network/CDN can be flaky)
        let mut last_error = None;
        for attempt in 1..=3 {
            log::info!("[Seeking] Attempt {}/3 to seek to {:?}", attempt, position);

            // Get redirect Location header without following - with retry logic
            let client = crate::utils::http::no_redirect_client();
            log::info!("[Seeking] Getting redirect Location header...");
            let response = match client
                .get(url)
                .header("Authorization", format!("OAuth {}", token))
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("[Seeking] Redirect request failed on attempt {}/3: {}", attempt, e);
                    last_error = Some(e.into());
                    if attempt < 3 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64)).await;
                    }
                    continue;
                }
            };
            
            // Extract Location header
            let actual_url = match response
                .headers()
                .get("location")
                .and_then(|h| h.to_str().ok())
            {
                Some(u) => u.to_string(),
                None => {
                    log::warn!("[Seeking] No Location header on attempt {}/3", attempt);
                    last_error = Some("No Location header in redirect".into());
                    if attempt < 3 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64)).await;
                    }
                    continue;
                }
            };
            
            log::info!("[Seeking] Got actual CDN URL from Location header");

            // STEP 4: Create NEW shutdown signal for new streaming thread
            self.shutdown_signal = Arc::new(AtomicBool::new(false));
            let shutdown_clone = Arc::clone(&self.shutdown_signal);

            // Create new streaming components with dual FFT channels
            let (sample_tx, sample_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
            let (fft_download_tx, fft_download_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
            let (fft_playback_tx, fft_playback_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
            let finished = Arc::new(Mutex::new(false));
            let finished_clone = Arc::clone(&finished);

            // STEP 5: Spawn new streaming thread and CAPTURE the JoinHandle
            let actual_url_clone = actual_url.clone();
            let stream_thread = std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    if let Err(e) = stream_from_actual_url(&actual_url_clone, byte_offset, sample_tx, fft_download_tx, finished_clone, shutdown_clone).await {
                        log::error!("[AudioPlayer] Seek streaming error: {}", e);
                    }
                });
            });

            // STEP 6: UPDATE the stream_thread field (critical!)
            self.stream_thread = Some(stream_thread);

            // Wait briefly for buffering
            std::thread::sleep(std::time::Duration::from_millis(100));

            let sample_rate = 44100;
            let channels = 2;

            // Create FFT analyzer only if FFT is enabled (GPU mode)
            let fft_enabled = bass_energy.is_some();
            if fft_enabled {
                if let (Some(bass), Some(mid), Some(high)) = (bass_energy.clone(), mid_energy.clone(), high_energy.clone()) {
                    let analyzer = crate::utils::audio_analyzer::AudioAnalyzer::new(
                        bass,
                        mid,
                        high,
                    );

                    let analyzer_arc = Arc::new(Mutex::new(analyzer));

                    // Spawn dedicated FFT processing thread for seek (merges download + playback)
                    let fft_analyzer = Arc::clone(&analyzer_arc);
                    std::thread::spawn(move || {
                log::info!("[FFT] Seek FFT processing thread started");
                
                // Process samples from both download and playback channels
                loop {
                    let mut got_sample = false;
                    
                    // Try download channel first (during buffering)
                    match fft_download_rx.try_recv() {
                        Ok(samples) => {
                            if let Ok(mut a) = fft_analyzer.lock() {
                                a.process_samples(&samples);
                            }
                            got_sample = true;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            // Download channel closed, that's normal
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            // No data yet
                        }
                    }
                    
                    // Try playback channel (during playback)
                    match fft_playback_rx.try_recv() {
                        Ok(samples) => {
                            if let Ok(mut a) = fft_analyzer.lock() {
                                a.process_samples(&samples);
                            }
                            got_sample = true;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            // Playback channel closed - track ended
                            log::info!("[FFT] Seek playback channel disconnected, ending FFT processing");
                            break;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            // No data yet
                        }
                    }
                    
                    // If no samples from either channel, sleep briefly
                    if !got_sample {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
                log::info!("[FFT] Seek FFT processing thread terminated");
                    });
                }
            } else {
                log::info!("[AudioPlayer] FFT disabled for seek - skipping FFT analyzer initialization");
            }

            // Audio source sends samples to FFT as they're played (only if FFT is enabled)
            let source = StreamingSource::new(
                sample_rx,
                sample_rate,
                channels,
                finished,
                if fft_enabled { Some(fft_playback_tx) } else { None },
                self.is_history_track,
                1.0, // network_quality_factor: 1.0 = good (TODO: pass from AudioState)
            );
            
            let new_sink = Sink::try_new(&self.stream_handle)?;
            new_sink.append(source);
            new_sink.set_volume(self.current_volume);

            self.sink = new_sink;
            self.start_position = position;
            self.start_time = Instant::now();
            self.paused_at = None;

            log::info!("[AudioPlayer] Seek completed successfully on attempt {}, streaming from {:?}", attempt, position);
            return Ok(());
        }
        
        // All retries failed
        Err(last_error.unwrap_or_else(|| "Seek failed after 3 attempts".into()))
    }
}

/// Stream from actual CDN URL with byte offset (for seeking)
async fn stream_from_actual_url(
    actual_url: &str,
    byte_offset: u64,
    sample_tx: Sender<Vec<i16>>,
    fft_tx: Sender<Vec<i16>>,
    finished: Arc<Mutex<bool>>,
    shutdown_signal: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = crate::utils::http::streaming_client();
    
    log::info!("[Streaming] Seeking to byte offset {} on CDN URL", byte_offset);
    
    // Retry seek requests up to 3 times
    let mut response = None;
    for attempt in 1..=3 {
        match client
            .get(actual_url)
            .header("Range", format!("bytes={}-", byte_offset))
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() || status.as_u16() == 206 {  // 206 = Partial Content
                    response = Some(resp);
                    break;
                } else {
                    log::warn!("[Streaming] Seek CDN returned status {} on attempt {}/3", status, attempt);
                    if attempt < 3 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64)).await;
                    }
                }
            }
            Err(e) => {
                log::warn!("[Streaming] Seek request failed on attempt {}/3: {}", attempt, e);
                if attempt < 3 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64)).await;
                }
            }
        }
    }
    
    let response = response.ok_or("Seek CDN request failed after 3 attempts")?;
    
    let mut mp3_buffer = Vec::new();
    let mut total_downloaded = byte_offset;
    let mut buffer_frames_sent = 0;
    
    use futures_util::StreamExt;

    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        // Check shutdown signal before processing each chunk
        if shutdown_signal.load(Ordering::Relaxed) {
            log::info!("[Streaming] Shutdown signal received, stopping seek download");
            return Ok(());
        }

        match chunk_result {
            Ok(chunk) => {
                mp3_buffer.extend_from_slice(&chunk);
                total_downloaded += chunk.len() as u64;
                
                // Decode all frames but only send new ones
                let mut decoder = Mp3Decoder::new(&mp3_buffer[..]);
                let mut frame_index = 0;
                
                loop {
                    match decoder.next_frame() {
                        Ok(Frame { data, .. }) => {
                            if frame_index >= buffer_frames_sent {
                                // Send to audio playback
                                if sample_tx.send(data.clone()).is_err() {
                                    log::debug!("[Streaming] Seek playback stopped");
                                    *finished.lock().unwrap() = true;
                                    return Ok(());
                                }
                                // Send to FFT (ignore errors - FFT is optional)
                                let _ = fft_tx.send(data);
                                buffer_frames_sent = frame_index + 1;
                            }
                            frame_index += 1;
                        }
                        Err(_) => break,
                    }
                }
                
                // Trim buffer if too large
                if mp3_buffer.len() > 5 * 1024 * 1024 {
                    let keep_size = 2 * 1024 * 1024;
                    let trim_amount = mp3_buffer.len() - keep_size;
                    mp3_buffer.drain(0..trim_amount);
                    buffer_frames_sent = 0;  // Reset - new buffer state
                }
            }
            Err(e) => {
                // Stream error mid-download - try to resume from current position
                log::warn!("[Streaming] Seek stream error at {} bytes: {} - attempting resume", total_downloaded, e);
                
                // Try to resume up to 2 times
                let mut resumed = false;
                for resume_attempt in 1..=2 {
                    log::info!("[Streaming] Resume attempt {}/2 from byte {}", resume_attempt, total_downloaded);
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    
                    match client
                        .get(actual_url)
                        .header("Range", format!("bytes={}-", total_downloaded))
                        .send()
                        .await
                    {
                        Ok(resume_response) => {
                            if resume_response.status().is_success() || resume_response.status().as_u16() == 206 {
                                log::info!("[Streaming] Successfully resumed stream from byte {}", total_downloaded);
                                stream = resume_response.bytes_stream();
                                resumed = true;
                                break;
                            } else {
                                log::warn!("[Streaming] Resume got status {}", resume_response.status());
                            }
                        }
                        Err(e) => {
                            log::warn!("[Streaming] Resume attempt {} failed: {}", resume_attempt, e);
                        }
                    }
                }
                
                if !resumed {
                    log::error!("[Streaming] Failed to resume stream after 2 attempts, giving up");
                    return Err(format!("Stream failed and could not resume from byte {}", total_downloaded).into());
                }
            }
        }
    }
    
    // Decode remaining frames from final buffer
    let mut decoder = Mp3Decoder::new(&mp3_buffer[..]);
    let mut frame_index = 0;
    while let Ok(Frame { data, .. }) = decoder.next_frame() {
        if frame_index >= buffer_frames_sent {
            let _ = sample_tx.send(data.clone());
            let _ = fft_tx.send(data);
        }
        frame_index += 1;
    }
    
    *finished.lock().unwrap() = true;
    Ok(())
}

/// Prefetch CDN redirect URL for a track (for auto-play optimization)
/// Returns the actual CDN URL that can be used for streaming
pub async fn prefetch_stream_url(
    stream_api_url: &str,
    token: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    log::info!("[Prefetch] Getting CDN URL for next track...");
    let client = crate::utils::http::no_redirect_client();

    // Retry up to 3 times on network errors
    for attempt in 1..=3 {
        match client
            .get(stream_api_url)
            .header("Authorization", format!("OAuth {}", token))
            .send()
            .await
        {
            Ok(response) => {
                // Extract Location header (the actual CDN URL)
                if let Some(location) = response.headers().get("location") {
                    if let Ok(cdn_url) = location.to_str() {
                        log::info!("[Prefetch] Successfully prefetched CDN URL (attempt {})", attempt);
                        return Ok(cdn_url.to_string());
                    }
                }
                return Err("No Location header in redirect".into());
            }
            Err(e) => {
                log::warn!("[Prefetch] Request failed on attempt {}/3: {}", attempt, e);
                if attempt < 3 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64)).await;
                } else {
                    return Err(format!("Failed to prefetch after 3 attempts: {}", e).into());
                }
            }
        }
    }

    Err("Prefetch failed after retries".into())
}

/// Stream audio data progressively and decode with minimp3
async fn stream_audio(
    url: &str,
    token: &str,
    _cache_key: &str,
    sample_tx: Sender<Vec<i16>>,
    fft_tx: Sender<Vec<i16>>,
    finished: Arc<Mutex<bool>>,
    shutdown_signal: Arc<AtomicBool>,
    prefetched_cdn_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Use prefetched CDN URL if available, otherwise fetch redirect
    let actual_url = if let Some(cdn_url) = prefetched_cdn_url {
        log::info!("[Streaming] Using prefetched CDN URL (skipping redirect fetch)");
        cdn_url
    } else {
        log::info!("[Streaming] Getting actual media URL from redirect...");
        let client = crate::utils::http::no_redirect_client();

        // Retry up to 3 times on network errors
        let mut response = None;
        for attempt in 1..=3 {
            match client
                .get(url)
                .header("Authorization", format!("OAuth {}", token))
                .send()
                .await
            {
                Ok(resp) => {
                    response = Some(resp);
                    break;
                }
                Err(e) => {
                    log::warn!("[Streaming] Redirect request failed on attempt {}/3: {}", attempt, e);
                    if attempt < 3 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64)).await;
                    } else {
                        return Err(format!("Failed to get stream redirect after 3 attempts: {}", e).into());
                    }
                }
            }
        }

        let response = response.ok_or("Failed to get redirect response")?;

        // Extract Location header
        response
            .headers()
            .get("location")
            .ok_or("No Location header in redirect")?
            .to_str()?
            .to_string()
    };
    
    log::info!("[Streaming] Streaming from actual URL: {}", actual_url);
    
    // Now stream from the actual CDN URL with retry logic
    let streaming_client = crate::utils::http::streaming_client();
    let mut streaming_response = None;
    
    // Retry up to 3 times on CDN errors
    for attempt in 1..=3 {
        match streaming_client.get(&actual_url).send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    streaming_response = Some(response);
                    break;
                } else {
                    log::warn!("[Streaming] CDN returned status {} on attempt {}/3", status, attempt);
                    if attempt < 3 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64)).await;
                    }
                }
            }
            Err(e) => {
                log::warn!("[Streaming] CDN request failed on attempt {}/3: {}", attempt, e);
                if attempt < 3 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64)).await;
                }
            }
        }
    }
    
    let streaming_response = streaming_response
        .ok_or("CDN failed after 3 attempts")?;
    
    // Get expected file size from Content-Length header (if available)
    let expected_size = streaming_response
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());
    
    if let Some(size) = expected_size {
        log::info!("[Streaming] Expected file size: {} KB ({} bytes)", size / 1024, size);
    } else {
        log::warn!("[Streaming] No Content-Length header - stream end detection may be less reliable");
    }
    
    let mut mp3_buffer = Vec::new();
    let mut total_downloaded = 0;
    let mut buffer_frames_sent = 0; // Track frames sent from CURRENT buffer state
    
    use futures_util::StreamExt;
    
    let mut stream = streaming_response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        // Check shutdown signal before processing each chunk
        if shutdown_signal.load(Ordering::Relaxed) {
            log::info!("[Streaming] Shutdown signal received, stopping download");
            return Ok(());
        }

        match chunk_result {
            Ok(chunk) => {
                mp3_buffer.extend_from_slice(&chunk);
                total_downloaded += chunk.len();

                // Decode all frames but only send new ones (original working method)
                let mut decoder = Mp3Decoder::new(&mp3_buffer[..]);
                let mut frame_index = 0;
                
                loop {
                    match decoder.next_frame() {
                        Ok(Frame { data, .. }) => {
                            // Only send frames we haven't sent yet from current buffer
                            if frame_index >= buffer_frames_sent {
                                // Send to audio playback
                                if sample_tx.send(data.clone()).is_err() {
                                    log::info!("[Streaming] Playback stopped by user, downloaded {} KB total", total_downloaded / 1024);
                                    *finished.lock().unwrap() = true;
                                    return Ok(());
                                }
                                // Send to FFT (ignore errors - FFT is optional)
                                let _ = fft_tx.send(data);
                                buffer_frames_sent = frame_index + 1;
                            }
                            frame_index += 1;
                        }
                        Err(_) => {
                            // No more complete frames available
                            break;
                        }
                    }
                }
                
                // Prevent excessive memory usage - trim old data if buffer > 5MB
                if mp3_buffer.len() > 5 * 1024 * 1024 {
                    // Keep last 2MB for frame continuity
                    let keep_size = 2 * 1024 * 1024;
                    let trim_amount = mp3_buffer.len() - keep_size;
                    mp3_buffer.drain(0..trim_amount);
                    // Reset counter - we're working with a new buffer now
                    buffer_frames_sent = 0;
                    log::debug!("[Streaming] Trimmed {} KB, buffer now {} KB, reset frame counter", trim_amount / 1024, mp3_buffer.len() / 1024);
                }

                if total_downloaded % (512 * 1024) == 0 {
                    log::debug!("[Streaming] Downloaded {} KB, buffer {} KB, sent {} frames from buffer...", 
                        total_downloaded / 1024, mp3_buffer.len() / 1024, buffer_frames_sent);
                }
            }
            Err(e) => {
                // Stream error mid-download - try to resume from current position
                log::warn!("[Streaming] Stream error at {} KB: {} - attempting resume", total_downloaded / 1024, e);
                
                // Try to resume up to 2 times
                let mut resumed = false;
                for resume_attempt in 1..=2 {
                    log::info!("[Streaming] Resume attempt {}/2 from byte {}", resume_attempt, total_downloaded);
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    
                    match streaming_client
                        .get(&actual_url)
                        .header("Range", format!("bytes={}-", total_downloaded))
                        .send()
                        .await
                    {
                        Ok(resume_response) => {
                            if resume_response.status().is_success() || resume_response.status().as_u16() == 206 {
                                log::info!("[Streaming] Successfully resumed stream from byte {}", total_downloaded);
                                stream = resume_response.bytes_stream();
                                resumed = true;
                                break;
                            } else {
                                log::warn!("[Streaming] Resume got status {}", resume_response.status());
                            }
                        }
                        Err(e) => {
                            log::warn!("[Streaming] Resume attempt {} failed: {}", resume_attempt, e);
                        }
                    }
                }
                
                if !resumed {
                    log::error!("[Streaming] Failed to resume stream after 2 attempts at {} KB, giving up", total_downloaded / 1024);
                    return Err(format!("Stream failed and could not resume from byte {}", total_downloaded).into());
                }
            }
        }
    }    // Stream complete - verify we got all the data
    if let Some(expected) = expected_size {
        let download_percent = (total_downloaded as f32 / expected as f32) * 100.0;
        if download_percent < 95.0 {
            log::warn!(
                "[Streaming] Stream ended prematurely! Downloaded {} KB / {} KB ({:.1}%)",
                total_downloaded / 1024,
                expected / 1024,
                download_percent
            );
        } else {
            log::info!(
                "[Streaming] Stream complete: {} KB / {} KB ({:.1}%)",
                total_downloaded / 1024,
                expected / 1024,
                download_percent
            );
        }
    } else {
        log::info!("[Streaming] Stream complete (no size validation available): {} KB total", total_downloaded / 1024);
    }
    
    // Decode any remaining frames we haven't sent from final buffer
    let mut decoder = Mp3Decoder::new(&mp3_buffer[..]);
    let mut frame_index = 0;
    while let Ok(Frame { data, .. }) = decoder.next_frame() {
        if frame_index >= buffer_frames_sent {
            let _ = sample_tx.send(data.clone());
            let _ = fft_tx.send(data);
        }
        frame_index += 1;
    }
    
    log::info!("[Streaming] Stream complete! Total downloaded: {} KB", total_downloaded / 1024);
    *finished.lock().unwrap() = true;
    Ok(())
}
