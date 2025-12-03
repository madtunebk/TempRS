use rodio::{OutputStream, Sink, Source};
use minimp3::{Decoder as Mp3Decoder, Frame};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

#[allow(dead_code)]
const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks for streaming

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
    #[allow(dead_code)]
    stream_thread: Option<std::thread::JoinHandle<()>>,
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
}

impl StreamingSource {
    fn new(sample_rx: Receiver<Vec<i16>>, sample_rate: u32, channels: u16, finished: Arc<Mutex<bool>>, fft_tx: Option<Sender<Vec<i16>>>) -> Self {
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
                
                if !self.current_samples.is_empty() {
                    let sample = self.current_samples[0];
                    self.sample_index = 1;
                    Some(sample)
                } else {
                    None
                }
            }
            Err(_) => {
                // Detect stream timeout (no data for 5 seconds)
                let timeout = self.last_sample_time.elapsed() > Duration::from_secs(5);
                
                // Check if streaming is finished
                let is_finished = *self.finished.lock().unwrap();
                
                if is_finished && !self.buffering {
                    // Stream ended cleanly after buffering completed
                    None
                } else if timeout {
                    // Stream stuck - force end to prevent infinite silence
                    log::error!("[StreamingSource] Stream timeout detected - ending playback");
                    *self.finished.lock().unwrap() = true;
                    None
                } else {
                    // Yield silence while waiting for more data (still buffering or stream active)
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
        bass_energy: std::sync::Arc<std::sync::Mutex<f32>>,
        mid_energy: std::sync::Arc<std::sync::Mutex<f32>>,
        high_energy: std::sync::Arc<std::sync::Mutex<f32>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("[AudioPlayer] Starting progressive streaming for track {}", track_id);
        let (_stream, stream_handle) = OutputStream::try_default()?;
        
        // Create dual channels - one for audio playback, one for FFT
        let (sample_tx, sample_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
        let (fft_download_tx, fft_download_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
        let (fft_playback_tx, fft_playback_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
        
        let finished = Arc::new(Mutex::new(false));
        let finished_clone = Arc::clone(&finished);
        
        let url_owned = url.to_string();
        let token_owned = token.to_string();
        let cache_key = format!("audio_{}", track_id);
        
        // Spawn streaming thread that sends to audio + FFT download channel
        let stream_thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = stream_audio(&url_owned, &token_owned, &cache_key, sample_tx, fft_download_tx, finished_clone).await {
                    log::error!("[AudioPlayer] Streaming error: {}", e);
                }
            });
        });
        
        // Wait briefly for first chunk to determine sample rate/channels
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        let sample_rate = 44100; // Default for MP3
        let channels = 2; // Stereo default
        
        // Create FFT analyzer that reads from its own dedicated channel
        let analyzer = crate::utils::audio_analyzer::AudioAnalyzer::new(
            Arc::clone(&bass_energy),
            Arc::clone(&mid_energy),
            Arc::clone(&high_energy),
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
        
        // Audio source sends samples to FFT as they're played (continues after download ends)
        let source = StreamingSource::new(sample_rx, sample_rate, channels, finished, Some(fft_playback_tx));
        
        let sink = Sink::try_new(&stream_handle)?;
        sink.append(source);
        log::info!("[AudioPlayer] Progressive streaming started - playing as we download!");
        
        Ok(Self {
            sink,
            _stream,
            stream_handle: stream_handle.clone(),
            total_duration: None, // Unknown for streaming
            start_time: Instant::now(),
            start_position: Duration::ZERO,
            paused_at: None,
            current_url: url.to_string(),
            current_token: token.to_string(),
            current_volume: 1.0,
            stream_thread: Some(stream_thread),
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
        log::debug!("[AudioPlayer] Stopping playback");
        self.sink.stop();
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.current_volume = volume;
        self.sink.set_volume(volume);
    }

    pub fn is_finished(&self) -> bool {
        self.sink.empty() && self.paused_at.is_none()
    }

    pub fn get_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    pub fn get_position(&self) -> Duration {
        if let Some(paused) = self.paused_at {
            paused
        } else {
            let elapsed = self.start_time.elapsed();
            let mut position = self.start_position.saturating_add(elapsed);
            if let Some(total) = self.total_duration {
                position = position.min(total);
            }
            position
        }
    }

    pub async fn seek(
        &mut self,
        position: Duration,
        url: &str,
        token: &str,
        bass_energy: Arc<Mutex<f32>>,
        mid_energy: Arc<Mutex<f32>>,
        high_energy: Arc<Mutex<f32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("[AudioPlayer] Seeking to {:?} by restarting stream...", position);

        // Stop current playback
        self.sink.stop();

        // Estimate byte offset (MP3 is typically 128kbps = 16KB/s)
        let bytes_per_second = 16_000;
        let byte_offset = position.as_secs() * bytes_per_second;

        // Get redirect Location header without following
        let client = crate::utils::http::no_redirect_client();
        log::info!("[Seeking] Getting redirect Location header...");
        let response = client
            .get(url)
            .header("Authorization", format!("OAuth {}", token))
            .send()
            .await?;
        
        // Extract Location header
        let actual_url = response
            .headers()
            .get("location")
            .ok_or("No Location header in redirect")?
            .to_str()?
            .to_string();
        
        log::info!("[Seeking] Got actual CDN URL from Location header");

        // Create new streaming components with dual FFT channels
        let (sample_tx, sample_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
        let (fft_download_tx, fft_download_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
        let (fft_playback_tx, fft_playback_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
        let finished = Arc::new(Mutex::new(false));
        let finished_clone = Arc::clone(&finished);
        
        // Spawn new streaming thread from offset using actual URL
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = stream_from_actual_url(&actual_url, byte_offset, sample_tx, fft_download_tx, finished_clone).await {
                    log::error!("[AudioPlayer] Seek streaming error: {}", e);
                }
            });
        });

        // Wait briefly for buffering
        std::thread::sleep(std::time::Duration::from_millis(100));

        let sample_rate = 44100;
        let channels = 2;
        
        // Create FFT analyzer for seek (same as initial playback)
        let analyzer = crate::utils::audio_analyzer::AudioAnalyzer::new(
            Arc::clone(&bass_energy),
            Arc::clone(&mid_energy),
            Arc::clone(&high_energy),
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
        
        let source = StreamingSource::new(sample_rx, sample_rate, channels, finished, Some(fft_playback_tx));
        
        let new_sink = Sink::try_new(&self.stream_handle)?;
        new_sink.append(source);
        new_sink.set_volume(self.current_volume);

        self.sink = new_sink;
        self.start_position = position;
        self.start_time = Instant::now();
        self.paused_at = None;

        log::info!("[AudioPlayer] Seek completed, streaming from {:?}", position);
        Ok(())
    }
}

/// Stream from actual CDN URL with byte offset (for seeking)
async fn stream_from_actual_url(
    actual_url: &str,
    byte_offset: u64,
    sample_tx: Sender<Vec<i16>>,
    fft_tx: Sender<Vec<i16>>,
    finished: Arc<Mutex<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = crate::utils::http::client();
    
    log::info!("[Streaming] Seeking to byte offset {} on CDN URL", byte_offset);
    let response = client
        .get(actual_url)
        .header("Range", format!("bytes={}-", byte_offset))
        .send()
        .await?;
    
    let mut mp3_buffer = Vec::new();
    let mut _total_downloaded = byte_offset;
    let mut total_frames_sent = 0;
    
    use futures_util::StreamExt;
    
    let mut stream = response.bytes_stream();
    
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        mp3_buffer.extend_from_slice(&chunk);
        _total_downloaded += chunk.len() as u64;
        
        // Decode all frames but only send new ones
        let mut decoder = Mp3Decoder::new(&mp3_buffer[..]);
        let mut frame_index = 0;
        
        loop {
            match decoder.next_frame() {
                Ok(Frame { data, .. }) => {
                    if frame_index >= total_frames_sent {
                        // Send to audio playback
                        if sample_tx.send(data.clone()).is_err() {
                            log::debug!("[Streaming] Seek playback stopped");
                            *finished.lock().unwrap() = true;
                            return Ok(());
                        }
                        // Send to FFT (ignore errors - FFT is optional)
                        let _ = fft_tx.send(data);
                        total_frames_sent += 1;
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
            total_frames_sent = 0;
        }
    }
    
    // Decode remaining
    let mut decoder = Mp3Decoder::new(&mp3_buffer[..]);
    let mut frame_index = 0;
    while let Ok(Frame { data, .. }) = decoder.next_frame() {
        if frame_index >= total_frames_sent {
            let _ = sample_tx.send(data.clone());
            let _ = fft_tx.send(data);
        }
        frame_index += 1;
    }
    
    *finished.lock().unwrap() = true;
    Ok(())
}

/// Stream audio data progressively and decode with minimp3
async fn stream_audio(
    url: &str,
    token: &str,
    _cache_key: &str,
    sample_tx: Sender<Vec<i16>>,
    fft_tx: Sender<Vec<i16>>,
    finished: Arc<Mutex<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get redirect Location header without following
    log::info!("[Streaming] Getting actual media URL from redirect...");
    let client = crate::utils::http::no_redirect_client();
    let response = client
        .get(url)
        .header("Authorization", format!("OAuth {}", token))
        .send()
        .await?;
    
    // Extract Location header
    let actual_url = response
        .headers()
        .get("location")
        .ok_or("No Location header in redirect")?
        .to_str()?
        .to_string();
    
    log::info!("[Streaming] Streaming from actual URL: {}", actual_url);
    
    // Now stream from the actual CDN URL
    let streaming_client = crate::utils::http::client();
    let streaming_response = streaming_client
        .get(&actual_url)
        .send()
        .await?;
    
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
    let mut total_frames_sent = 0; // Track frames we've already sent
    
    use futures_util::StreamExt;
    
    let mut stream = streaming_response.bytes_stream();
    
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        mp3_buffer.extend_from_slice(&chunk);
        total_downloaded += chunk.len();
        
        // Decode all frames but only send new ones (original working method)
        let mut decoder = Mp3Decoder::new(&mp3_buffer[..]);
        let mut frame_index = 0;
        
        loop {
            match decoder.next_frame() {
                Ok(Frame { data, .. }) => {
                    // Only send frames we haven't sent yet
                    if frame_index >= total_frames_sent {
                        // Send to audio playback
                        if sample_tx.send(data.clone()).is_err() {
                            log::info!("[Streaming] Playback stopped by user, downloaded {} KB total", total_downloaded / 1024);
                            *finished.lock().unwrap() = true;
                            return Ok(());
                        }
                        // Send to FFT (ignore errors - FFT is optional)
                        let _ = fft_tx.send(data);
                        total_frames_sent += 1;
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
            total_frames_sent = 0;  // Reset counter since we trimmed
            log::debug!("[Streaming] Trimmed buffer to {} KB", mp3_buffer.len() / 1024);
        }
        
        if total_downloaded % (512 * 1024) == 0 {
            log::debug!("[Streaming] Downloaded {} KB, buffer {} KB, sent {} frames...", 
                total_downloaded / 1024, mp3_buffer.len() / 1024, total_frames_sent);
        }
    }
    
    // Stream complete - verify we got all the data
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
    
    // Decode any remaining frames we haven't sent
    let mut decoder = Mp3Decoder::new(&mp3_buffer[..]);
    let mut frame_index = 0;
    while let Ok(Frame { data, .. }) = decoder.next_frame() {
        if frame_index >= total_frames_sent {
            let _ = sample_tx.send(data.clone());
            let _ = fft_tx.send(data);
        }
        frame_index += 1;
    }
    
    log::info!("[Streaming] Stream complete! Total downloaded: {} KB", total_downloaded / 1024);
    *finished.lock().unwrap() = true;
    Ok(())
}
