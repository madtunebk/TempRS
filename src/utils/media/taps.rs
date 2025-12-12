use std::sync::{
    mpsc::{channel, Receiver, Sender, TryRecvError},
    Arc, Mutex,
};
use std::thread::JoinHandle;

/// Dual-channel FFT tap: accepts samples from download and playback channels.
pub struct DualFftTap {
    pub download_tx: Sender<Vec<i16>>,
    pub playback_tx: Sender<Vec<i16>>,
    pub _thread: JoinHandle<()>,
}

impl DualFftTap {
    pub fn new(
        bass: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        mid: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        high: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
    ) -> Option<Self> {
        if let (Some(b), Some(m), Some(h)) = (bass, mid, high) {
            let (download_tx, download_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
            let (playback_tx, playback_rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();
            let analyzer = crate::utils::audio_analyzer::AudioAnalyzer::new(b, m, h);
            let analyzer = Arc::new(Mutex::new(analyzer));
            let thread = {
                let analyzer = analyzer.clone();
                std::thread::spawn(move || {
                    loop {
                        let mut got = false;
                        match download_rx.try_recv() {
                            Ok(samples) => {
                                if let Ok(mut a) = analyzer.lock() {
                                    a.process_samples(&samples);
                                }
                                got = true;
                            }
                            Err(TryRecvError::Disconnected) => { /* ok */ }
                            Err(TryRecvError::Empty) => {}
                        }
                        match playback_rx.try_recv() {
                            Ok(samples) => {
                                if let Ok(mut a) = analyzer.lock() {
                                    a.process_samples(&samples);
                                }
                                got = true;
                            }
                            Err(TryRecvError::Disconnected) => { /* ok */ }
                            Err(TryRecvError::Empty) => {}
                        }
                        if !got {
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        // No explicit exit condition; thread terminates when process exits or channel errors escalate
                    }
                })
            };
            Some(Self {
                download_tx,
                playback_tx,
                _thread: thread,
            })
        } else {
            None
        }
    }
}
