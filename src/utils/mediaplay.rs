use rodio::{OutputStream, Sink, Source};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{
    mpsc::{channel, Receiver, Sender, TryRecvError},
    Arc,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

// Idle handling is managed in utils::media::core

// -----------------------------------------------------------------------------
// Streaming Source (rodio::Source implementation)
// -----------------------------------------------------------------------------

struct StreamingSource {
    sample_rx: Receiver<Vec<i16>>,
    current: Vec<i16>,
    idx: usize,
    sample_rate: u32,
    channels: u16,
    playback_fft_tx: Option<Sender<Vec<i16>>>,
    playback_fft_buf: Vec<i16>,
}

impl StreamingSource {
    fn new(
        sample_rx: Receiver<Vec<i16>>,
        sample_rate: u32,
        channels: u16,
        playback_fft_tx: Option<Sender<Vec<i16>>>,
    ) -> Self {
        Self {
            sample_rx,
            current: Vec::new(),
            idx: 0,
            sample_rate,
            channels,
            playback_fft_tx,
            playback_fft_buf: Vec::with_capacity(1152),
        }
    }
}

impl Iterator for StreamingSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.current.len() {
            let s = self.current[self.idx];
            self.idx += 1;
            if let Some(tx) = &self.playback_fft_tx {
                self.playback_fft_buf.push(s);
                if self.playback_fft_buf.len() >= 1152 {
                    let _ = tx.send(self.playback_fft_buf.clone());
                    self.playback_fft_buf.clear();
                }
            }
            return Some(s);
        }

        match self.sample_rx.try_recv() {
            Ok(samples) => {
                self.current = samples;
                self.idx = 0;
                if self.current.is_empty() {
                    return None;
                }
                let s = self.current[0];
                self.idx = 1;
                if let Some(tx) = &self.playback_fft_tx {
                    self.playback_fft_buf.push(s);
                    if self.playback_fft_buf.len() >= 1152 {
                        let _ = tx.send(self.playback_fft_buf.clone());
                        self.playback_fft_buf.clear();
                    }
                }
                Some(s)
            }
            Err(TryRecvError::Empty) => {
                // No new data yet; output silence to avoid glitches
                Some(0)
            }
            Err(TryRecvError::Disconnected) => {
                // Producer ended; finish gracefully
                None
            }
        }
    }
}

impl Source for StreamingSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// -----------------------------------------------------------------------------
// Audio Player (owned by AudioController thread)
// -----------------------------------------------------------------------------

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
    stream_thread: Option<std::thread::JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    #[allow(dead_code)]
    fft_thread: Option<JoinHandle<()>>,
}

impl AudioPlayer {
    #[allow(clippy::too_many_arguments)]
    pub async fn new_and_play_cached(
        url: &str,
        token: &str,
        _track_id: u64,
        duration_ms: u64,
        bass_energy: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        mid_energy: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        high_energy: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        _is_history_track: bool,
        prefetched_cdn_url: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (stream, handle) = OutputStream::try_default()?;

        let (tx, rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
        // Optional dual FFT tap (download + playback)
        let fft_tap =
            crate::utils::media::taps::DualFftTap::new(bass_energy, mid_energy, high_energy);
        let shutdown = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let shutdown_cl = shutdown.clone();
        let finished_cl = finished.clone();

        // Spawn streaming thread
        let url_owned = url.to_string();
        let token_owned = token.to_string();
        let download_tx_opt = fft_tap.as_ref().map(|t| t.download_tx.clone());
        let stream_thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                if let Err(e) = stream_audio_simple(
                    &url_owned,
                    &token_owned,
                    tx,
                    download_tx_opt,
                    shutdown_cl,
                    finished_cl,
                    prefetched_cdn_url,
                )
                .await
                {
                    log::error!("[AudioPlayer] Streaming error: {}", e);
                }
            });
        });

        let source = StreamingSource::new(
            rx,
            44100,
            2,
            fft_tap.as_ref().map(|t| t.playback_tx.clone()),
        );
        let sink = Sink::try_new(&handle)?;
        sink.append(source);

        let total_duration = if duration_ms > 0 {
            Some(Duration::from_millis(duration_ms))
        } else {
            None
        };

        Ok(Self {
            sink,
            _stream: stream,
            stream_handle: handle,
            total_duration,
            start_time: Instant::now(),
            start_position: Duration::ZERO,
            paused_at: None,
            current_url: url.to_string(),
            current_token: token.to_string(),
            current_volume: 1.0,
            stream_thread: Some(stream_thread),
            shutdown,
            finished,
            fft_thread: fft_tap.map(|t| t._thread),
        })
    }

    pub fn pause(&mut self) {
        if !self.sink.is_paused() {
            self.paused_at = Some(self.get_position());
            self.sink.pause();
        }
    }

    pub fn resume(&mut self) {
        if self.sink.is_paused() {
            if let Some(p) = self.paused_at.take() {
                self.start_position = p;
                self.start_time = Instant::now();
            }
            self.sink.play();
        }
    }

    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.sink.stop();
        if let Some(handle) = self.stream_thread.take() {
            // Detach cleanup so we never block here (network may be mid-await)
            std::thread::spawn(move || {
                let start = std::time::Instant::now();
                while !handle.is_finished() && start.elapsed() < std::time::Duration::from_secs(2) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                let _ = handle.join();
            });
        }
        if let Some(h) = self.fft_thread.take() {
            std::thread::spawn(move || {
                let _ = h.join();
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
        if !self.sink.empty() {
            return false;
        }
        // Sink is empty; if stream ended or we’re near the end, consider finished
        if self.finished.load(Ordering::Relaxed) {
            return true;
        }
        if let Some(total) = self.total_duration {
            let remaining = total.saturating_sub(self.get_position());
            return remaining <= Duration::from_secs(2);
        }
        true
    }

    pub fn get_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    pub fn get_position(&self) -> Duration {
        if let Some(p) = self.paused_at {
            return p;
        }
        let elapsed = self.start_time.elapsed();
        let pos = self.start_position.saturating_add(elapsed);
        if let Some(total) = self.total_duration {
            pos.min(total)
        } else {
            pos
        }
    }

    pub async fn seek(
        &mut self,
        position: Duration,
        url: &str,
        token: &str,
        bass_energy: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        mid_energy: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        high_energy: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // stop old stream
        self.shutdown.store(true, Ordering::Relaxed);
        self.sink.stop();
        if let Some(handle) = self.stream_thread.take() {
            std::thread::spawn(move || {
                let start = std::time::Instant::now();
                while !handle.is_finished() && start.elapsed() < std::time::Duration::from_secs(2) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                let _ = handle.join();
            });
        }

        // start new stream from offset
        let (tx, rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
        self.shutdown = Arc::new(AtomicBool::new(false));
        self.finished = Arc::new(AtomicBool::new(false));
        let shutdown_cl = self.shutdown.clone();
        let finished_cl = self.finished.clone();
        let url_owned = url.to_string();
        let token_owned = token.to_string();
        let byte_offset = position.as_secs() * 16_000; // rough 128kbps

        // Optional FFT pipeline for seek
        let fft_tap =
            crate::utils::media::taps::DualFftTap::new(bass_energy, mid_energy, high_energy);
        // For seek, we don't retain a separate handle; analyzer thread exits when senders drop
        self.fft_thread = None;

        let download_tx_opt = fft_tap.as_ref().map(|t| t.download_tx.clone());
        let stream_thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                if let Err(e) = stream_audio_from_offset(
                    &url_owned,
                    &token_owned,
                    tx,
                    download_tx_opt,
                    shutdown_cl,
                    finished_cl,
                    byte_offset,
                )
                .await
                {
                    log::error!("[AudioPlayer] Seek streaming error: {}", e);
                }
            });
        });

        let source = StreamingSource::new(
            rx,
            44100,
            2,
            fft_tap.as_ref().map(|t| t.playback_tx.clone()),
        );
        let new_sink = Sink::try_new(&self.stream_handle)?;
        new_sink.append(source);
        new_sink.set_volume(self.current_volume);
        self.sink = new_sink;
        self.start_position = position;
        self.start_time = Instant::now();
        self.paused_at = None;
        self.stream_thread = Some(stream_thread);
        // Note: FFT thread recreated inside start_fft_pipeline; old thread ends when sender drops
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Simple streaming helpers
// -----------------------------------------------------------------------------

// Dual FFT tap moved to utils::media::taps

async fn stream_audio_simple(
    api_url: &str,
    token: &str,
    sample_tx: Sender<Vec<i16>>,
    fft_tx: Option<Sender<Vec<i16>>>,
    shutdown: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    prefetched_cdn_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let actual_url = match prefetched_cdn_url {
        Some(url) => {
            log::info!("[Streaming] Using prefetched CDN URL");
            url
        }
        None => crate::utils::stream_utils::resolve_redirect(api_url, token)
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(std::io::Error::other(e)) })?,
    };
    crate::utils::media::core::stream_from_cdn(
        &actual_url,
        None,
        sample_tx,
        fft_tx,
        shutdown,
        finished,
    )
    .await
}

async fn stream_audio_from_offset(
    api_url: &str,
    token: &str,
    sample_tx: Sender<Vec<i16>>,
    fft_tx: Option<Sender<Vec<i16>>>,
    shutdown: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    byte_offset: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let actual_url = crate::utils::stream_utils::resolve_redirect(api_url, token)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(std::io::Error::other(e)) })?;
    crate::utils::media::core::stream_from_cdn(
        &actual_url,
        Some(byte_offset),
        sample_tx,
        fft_tx,
        shutdown,
        finished,
    )
    .await
}

// Redirect resolution moved to utils::stream_utils

// Streaming core moved to utils::media::core

// Prefetch moved to utils::stream_utils; streaming core moved to utils::media::core
