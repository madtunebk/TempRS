/// Real-time FFT audio analysis for visualizer
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::{Arc, Mutex};

// ============================================================================
// FFT TUNING CONSTANTS - Adjust these to fine-tune visualizer behavior
// ============================================================================

const FFT_SIZE: usize = 2048;  // Good balance between frequency resolution and latency
const SAMPLE_RATE: f32 = 44100.0;  // CD quality

// Energy scaling (INCREASE to lower bar heights, DECREASE to raise them)
const BASS_SCALE: f32 = 2500.0;  // Bass normalization (was 1000.0, higher = lower bars)
const MID_SCALE: f32  = 2500.0;  // Mid normalization
const HIGH_SCALE: f32 = 2500.0;  // High normalization

// Smoothing (0.0 = instant changes, 1.0 = very smooth/laggy)
const SMOOTHING_OLD: f32 = 0.3;  // Weight for old value (0.3 = 30% old)
const SMOOTHING_NEW: f32 = 0.7;  // Weight for new value (0.7 = 70% new)

// ============================================================================

/// Audio analyzer that performs FFT on incoming audio samples
pub struct AudioAnalyzer {
    buffer: Vec<f32>,
    bass_energy: Arc<Mutex<f32>>,
    mid_energy: Arc<Mutex<f32>>,
    high_energy: Arc<Mutex<f32>>,
}

impl AudioAnalyzer {
    pub fn new(
        bass_energy: Arc<Mutex<f32>>,
        mid_energy: Arc<Mutex<f32>>,
        high_energy: Arc<Mutex<f32>>,
    ) -> Self {
        Self {
            buffer: Vec::with_capacity(FFT_SIZE),
            bass_energy,
            mid_energy,
            high_energy,
        }
    }

    /// Process incoming audio samples (mono, i16 -> f32)
    pub fn process_samples(&mut self, samples: &[i16]) {
        // Convert i16 samples to f32 and add to buffer
        for &sample in samples {
            self.buffer.push(sample as f32 / 32768.0);
            
            // When buffer is full, run FFT
            if self.buffer.len() >= FFT_SIZE {
                self.run_fft();
                // Keep last half for overlap (smoother transitions)
                self.buffer.drain(0..FFT_SIZE / 2);
            }
        }
    }

    /// Run FFT and extract frequency bands
    fn run_fft(&mut self) {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        // Convert to complex numbers
        let mut buffer: Vec<Complex<f32>> = self.buffer
            .iter()
            .take(FFT_SIZE)
            .map(|&x| Complex { re: x, im: 0.0 })
            .collect();

        // Apply Hann window to reduce spectral leakage
        for (i, sample) in buffer.iter_mut().enumerate() {
            let window = 0.5 * (1.0 - ((2.0 * std::f32::consts::PI * i as f32) / (FFT_SIZE as f32 - 1.0)).cos());
            sample.re *= window;
        }

        // Perform FFT
        fft.process(&mut buffer);

        // Calculate frequency bin size
        let bin_hz = SAMPLE_RATE / FFT_SIZE as f32;

        // Define frequency ranges (in Hz)
        // Bass: 20-250 Hz (sub-bass + bass)
        // Mid: 250-4000 Hz (midrange + presence)
        // High: 4000-20000 Hz (brilliance)
        let bass_bins = (20.0 / bin_hz) as usize..(250.0 / bin_hz) as usize;
        let mid_bins = (250.0 / bin_hz) as usize..(4000.0 / bin_hz) as usize;
        let high_bins = (4000.0 / bin_hz) as usize..(20000.0 / bin_hz) as usize;

        // Calculate energy in each band (magnitude)
        let bass = self.calculate_band_energy(&buffer, bass_bins);
        let mid = self.calculate_band_energy(&buffer, mid_bins);
        let high = self.calculate_band_energy(&buffer, high_bins);

        // Normalize using tuning constants (see top of file)
        let bass_norm = (bass / BASS_SCALE).min(1.0);
        let mid_norm = (mid / MID_SCALE).min(1.0);
        let high_norm = (high / HIGH_SCALE).min(1.0);

        // Update shared values with smoothing (prevents jitter)
        if let Ok(mut b) = self.bass_energy.lock() {
            *b = *b * SMOOTHING_OLD + bass_norm * SMOOTHING_NEW;
        }
        if let Ok(mut m) = self.mid_energy.lock() {
            *m = *m * SMOOTHING_OLD + mid_norm * SMOOTHING_NEW;
        }
        if let Ok(mut h) = self.high_energy.lock() {
            *h = *h * SMOOTHING_OLD + high_norm * SMOOTHING_NEW;
        }
    }

    /// Calculate total energy in a frequency band
    fn calculate_band_energy(&self, fft_buffer: &[Complex<f32>], range: std::ops::Range<usize>) -> f32 {
        let mut energy = 0.0;
        for i in range {
            if i < fft_buffer.len() / 2 {  // Only use first half (positive frequencies)
                let magnitude = (fft_buffer[i].re.powi(2) + fft_buffer[i].im.powi(2)).sqrt();
                energy += magnitude;
            }
        }
        energy
    }
}
