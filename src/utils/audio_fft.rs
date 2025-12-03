// Audio FFT module - currently unused but kept for future audio visualization features
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
const FFT_SIZE: usize = 2048;
#[allow(dead_code)]
const SAMPLE_RATE: f32 = 44100.0;
#[allow(dead_code)]
pub const NUM_FREQUENCY_BANDS: usize = 64; // Number of frequency bands for visualization

/// Audio FFT analyzer that processes audio samples and produces frequency spectrum
#[allow(dead_code)]
pub struct AudioFFT {
    sample_buffer: Arc<Mutex<Vec<f32>>>,
    frequency_data: Arc<Mutex<Vec<f32>>>,
    planner: FftPlanner<f32>,
}

#[allow(dead_code)]
impl AudioFFT {
    pub fn new() -> Self {
        Self {
            sample_buffer: Arc::new(Mutex::new(Vec::with_capacity(FFT_SIZE))),
            frequency_data: Arc::new(Mutex::new(vec![0.0; NUM_FREQUENCY_BANDS])),
            planner: FftPlanner::new(),
        }
    }

    /// Get a cloneable handle to the sample buffer for audio source to write to
    pub fn get_sample_buffer(&self) -> Arc<Mutex<Vec<f32>>> {
        self.sample_buffer.clone()
    }

    /// Get a cloneable handle to read frequency data
    pub fn get_frequency_data(&self) -> Arc<Mutex<Vec<f32>>> {
        self.frequency_data.clone()
    }

    /// Add audio samples to the buffer (called from audio thread)
    pub fn push_samples(&self, samples: &[i16]) {
        let mut buffer = self.sample_buffer.lock().unwrap();
        
        // Convert i16 samples to f32 and normalize
        for &sample in samples {
            let normalized = sample as f32 / 32768.0;
            buffer.push(normalized);
            
            // Keep buffer size manageable
            if buffer.len() > FFT_SIZE * 2 {
                let drain_count = buffer.len() - FFT_SIZE;
                buffer.drain(0..drain_count);
            }
        }
    }

    /// Process current samples and update frequency data
    pub fn update(&mut self) {
        let buffer = self.sample_buffer.lock().unwrap();
        
        if buffer.len() < FFT_SIZE {
            return; // Not enough samples yet
        }

        // Take the last FFT_SIZE samples
        let samples: Vec<f32> = buffer.iter().rev().take(FFT_SIZE).rev().copied().collect();
        drop(buffer); // Release lock early

        // Apply Hanning window to reduce spectral leakage
        let mut windowed: Vec<Complex<f32>> = samples
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let window = 0.5 * (1.0 - f32::cos(2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE as f32 - 1.0)));
                Complex::new(s * window, 0.0)
            })
            .collect();

        // Perform FFT
        let fft = self.planner.plan_fft_forward(FFT_SIZE);
        fft.process(&mut windowed);

        // Convert to magnitude spectrum and group into bands
        let mut freq_data = self.frequency_data.lock().unwrap();
        
        // We only care about first half of FFT (Nyquist)
        let bin_per_band = (FFT_SIZE / 2) / NUM_FREQUENCY_BANDS;
        
        for band in 0..NUM_FREQUENCY_BANDS {
            let start_bin = band * bin_per_band;
            let end_bin = (start_bin + bin_per_band).min(FFT_SIZE / 2);
            
            let mut magnitude = 0.0;
            for bin in start_bin..end_bin {
                magnitude += windowed[bin].norm();
            }
            magnitude /= bin_per_band as f32;
            
            // Apply smoothing and logarithmic scaling
            let smoothing = 0.7; // Higher = smoother
            freq_data[band] = freq_data[band] * smoothing + magnitude * (1.0 - smoothing);
            
            // Normalize to 0-1 range with some headroom
            freq_data[band] = (freq_data[band] * 2.0).min(1.0);
        }
    }

    /// Get current frequency band values (0.0 to 1.0)
    pub fn get_bands(&self) -> Vec<f32> {
        self.frequency_data.lock().unwrap().clone()
    }
}

impl Default for AudioFFT {
    fn default() -> Self {
        Self::new()
    }
}
