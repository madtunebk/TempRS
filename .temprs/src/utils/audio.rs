use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

#[derive(Default)]
pub struct AudioState {
    bass: Mutex<f32>,
    mid: Mutex<f32>,
    high: Mutex<f32>,
}

impl AudioState {
    #[allow(dead_code)]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    #[allow(dead_code)]
    pub fn set_bands(&self, bass: f32, mid: f32, high: f32) {
        if let Ok(mut b) = self.bass.lock() {
            *b = bass.clamp(0.0, 1.0);
        }
        if let Ok(mut m) = self.mid.lock() {
            *m = mid.clamp(0.0, 1.0);
        }
        if let Ok(mut h) = self.high.lock() {
            *h = high.clamp(0.0, 1.0);
        }
    }

    #[allow(dead_code)]
    pub fn get_bands(&self) -> (f32, f32, f32) {
        let b = self.bass.lock().ok().map(|g| *g).unwrap_or(0.0);
        let m = self.mid.lock().ok().map(|g| *g).unwrap_or(0.0);
        let h = self.high.lock().ok().map(|g| *g).unwrap_or(0.0);
        (b, m, h)
    }
}

// Starts capturing audio from the default input device and computes a basic
// FFT-based 3-band spectrum (bass/mid/high). Updates `AudioState` continuously.
// Returns an optional `cpal::Stream` handle which must be kept alive.
#[allow(dead_code)]
pub fn start_input_fft(audio: Arc<AudioState>) -> Option<cpal::Stream> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    info!(">>> start_input_fft() called!");
    
    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => {
            info!("Audio input device found: {:?}", d.name());
            d
        }
        None => {
            warn!("No default input device");
            return None;
        }
    };
    let config = match device.default_input_config() {
        Ok(c) => {
            info!("Audio config - sample_rate: {}, channels: {}, format: {:?}", 
                  c.sample_rate().0, c.channels(), c.sample_format());
            c
        }
        Err(e) => {
            warn!("No default input config: {e}");
            return None;
        }
    };

    let sample_rate = config.sample_rate().0 as usize;
    let channels = config.channels() as usize;

    // Use a modest FFT size; power of two for rustfft
    const FFT_LEN: usize = 1024;
    let (tx, rx) = mpsc::channel::<f32>();

    // Analyzer thread: aggregates samples, runs FFT, smooths bands, updates state
    std::thread::spawn(move || {
        use rustfft::{num_complex::Complex32, FftPlanner};

        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_LEN);
        let window: Vec<f32> = (0..FFT_LEN)
            .map(|n| 0.5 - 0.5 * ((2.0 * std::f32::consts::PI * n as f32) / FFT_LEN as f32).cos())
            .collect();
        let mut buf: Vec<Complex32> = vec![Complex32::new(0.0, 0.0); FFT_LEN];
        let mut tmp: Vec<f32> = vec![0.0; FFT_LEN];

        // Smoothing state
        let mut sb = 0.0f32; // bass
        let mut sm = 0.0f32; // mid
        let mut sh = 0.0f32; // high
        let attack = 0.5f32; // faster rise
        let decay = 0.2f32; // slower fall
        
        let mut frame_count = 0usize;
        info!("Audio FFT thread started");

        loop {
            // Fill tmp with mono samples
            for sample in tmp.iter_mut().take(FFT_LEN) {
                // Accumulate across channels -> mono
                let mut acc = 0.0f32;
                for _c in 0..channels {
                    let v = match rx.recv() {
                        Ok(v) => v,
                        Err(_) => return,
                    }; // stream ended
                    acc += v;
                }
                *sample = acc / channels as f32;
            }

            // Apply window and copy to complex buffer
            for i in 0..FFT_LEN {
                buf[i].re = tmp[i] * window[i];
                buf[i].im = 0.0;
            }
            fft.process(&mut buf);

            // Compute magnitudes for positive frequencies
            let nyquist = sample_rate as f32 / 2.0;
            let bin_hz = nyquist / (FFT_LEN as f32 / 2.0);
            let mut bass_e = 0.0f32;
            let mut mid_e = 0.0f32;
            let mut high_e = 0.0f32;
            let mut bass_n = 0usize;
            let mut mid_n = 0usize;
            let mut high_n = 0usize;
            for (i, c) in buf.iter().take(FFT_LEN / 2).enumerate().skip(1) {
                let hz = i as f32 * bin_hz;
                let mag = (c.re * c.re + c.im * c.im).sqrt();
                if hz < 250.0 {
                    bass_e += mag;
                    bass_n += 1;
                } else if hz < 2000.0 {
                    mid_e += mag;
                    mid_n += 1;
                } else {
                    high_e += mag;
                    high_n += 1;
                }
            }
            let mut b = if bass_n > 0 {
                bass_e / bass_n as f32
            } else {
                0.0
            };
            let mut m = if mid_n > 0 { mid_e / mid_n as f32 } else { 0.0 };
            let mut h = if high_n > 0 {
                high_e / high_n as f32
            } else {
                0.0
            };

            // Normalize roughly (depends on input gain)
            let norm = 0.1f32; // empirical scale
            b *= norm;
            m *= norm;
            h *= norm;
            b = b.clamp(0.0, 1.0);
            m = m.clamp(0.0, 1.0);
            h = h.clamp(0.0, 1.0);

            // Smooth
            sb = if b > sb {
                sb + attack * (b - sb)
            } else {
                sb + decay * (b - sb)
            };
            sm = if m > sm {
                sm + attack * (m - sm)
            } else {
                sm + decay * (m - sm)
            };
            sh = if h > sh {
                sh + attack * (h - sh)
            } else {
                sh + decay * (h - sh)
            };

            audio.set_bands(sb, sm, sh);
            
            // Debug print every 30 frames (~1 second at typical rates)
            frame_count += 1;
            if frame_count.is_multiple_of(30) {
                debug!("Audio bands - bass: {:.3}, mid: {:.3}, high: {:.3}", sb, sm, sh);
            }
        }
    });

    // Build and start the CPAL input stream
    let cfg: cpal::StreamConfig = config.clone().into();
    info!("Building audio input stream...");
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_input_stream_f32(&device, &cfg, tx),
        cpal::SampleFormat::I16 => build_input_stream_i16(&device, &cfg, tx),
        cpal::SampleFormat::U16 => build_input_stream_u16(&device, &cfg, tx),
        _ => {
            warn!("Unsupported sample format");
            return None;
        }
    }
    .ok()?;

    if let Err(e) = stream.play() {
        warn!("Failed to start input stream: {e}");
        return None;
    }
    info!("Audio input stream started successfully");
    Some(stream)
}

fn build_input_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: Sender<f32>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    use cpal::traits::DeviceTrait;
    let channels = config.channels as usize;
    let mut sample_count = 0usize;
    device.build_input_stream(
        config,
        move |data: &[f32], _| {
            sample_count += data.len();
            if sample_count.is_multiple_of(48000) {
                debug!("Received {} audio samples", sample_count);
            }
            for frame in data.chunks_exact(channels) {
                for &s in frame {
                    let _ = tx.send(s);
                }
            }
        },
        move |err| warn!("Audio input error: {err}"),
        None,
    )
}

fn build_input_stream_i16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: Sender<f32>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    use cpal::traits::DeviceTrait;
    let channels = config.channels as usize;
    device.build_input_stream(
        config,
        move |data: &[i16], _| {
            for frame in data.chunks_exact(channels) {
                for &s in frame {
                    let _ = tx.send(s as f32 / i16::MAX as f32);
                }
            }
        },
        move |err| warn!("Audio input error: {err}"),
        None,
    )
}

fn build_input_stream_u16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: Sender<f32>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    use cpal::traits::DeviceTrait;
    let channels = config.channels as usize;
    device.build_input_stream(
        config,
        move |data: &[u16], _| {
            for frame in data.chunks_exact(channels) {
                for &s in frame {
                    let _ = tx.send((s as f32 / u16::MAX as f32) * 2.0 - 1.0);
                }
            }
        },
        move |err| warn!("Audio input error: {err}"),
        None,
    )
}
