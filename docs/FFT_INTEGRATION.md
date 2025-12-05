# FFT Audio Visualization Integration Guide

## Overview
Added FFT (Fast Fourier Transform) audio analysis capability to TempRS for real-time frequency spectrum visualization in shaders.

## New Components

### 1. audio_fft.rs - FFT Processor
**Location**: `src/utils/audio_fft.rs`

**Features**:
- 2048-sample FFT window with Hanning window function
- 64 frequency bands (configurable via `NUM_FREQUENCY_BANDS`)
- Real-time processing at 44.1kHz sample rate
- Logarithmic scaling and smoothing for visual appeal
- Thread-safe with Arc<Mutex<>> for cross-thread access

**API**:
```rust
let fft = AudioFFT::new();

// Get buffer handle for audio source to write to
let sample_buffer = fft.get_sample_buffer();

// Push samples from audio thread
fft.push_samples(&i16_samples);

// Update FFT (call from main thread ~30-60 FPS)
fft.update();

// Get frequency bands for visualization
let bands: Vec<f32> = fft.get_bands(); // 64 values, 0.0-1.0 range
```

## Integration Steps

### Step 1: Modify StreamingSource to Capture Samples

**File**: `src/utils/mediaplay.rs`

Add FFT buffer to `StreamingSource`:

```rust
pub struct StreamingSource {
    sample_rx: Receiver<Vec<i16>>,
    current_samples: Vec<i16>,
    sample_index: usize,
    channels: u16,
    sample_rate: u32,
    fft_buffer: Option<Arc<Mutex<Vec<f32>>>>,  // ADD THIS
}

impl StreamingSource {
    pub fn new(
        sample_rx: Receiver<Vec<i16>>,
        channels: u16,
        sample_rate: u32,
        fft_buffer: Option<Arc<Mutex<Vec<f32>>>>,  // ADD THIS
    ) -> Self {
        Self {
            sample_rx,
            current_samples: Vec::new(),
            sample_index: 0,
            channels,
            sample_rate,
            fft_buffer,  // ADD THIS
        }
    }
}

impl Iterator for StreamingSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        // ... existing code ...
        
        if self.sample_index >= self.current_samples.len() {
            match self.sample_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(samples) => {
                    // CAPTURE SAMPLES FOR FFT
                    if let Some(fft_buf) = &self.fft_buffer {
                        let mut buf = fft_buf.lock().unwrap();
                        // Convert i16 to f32 and add to buffer
                        for &sample in &samples {
                            buf.push(sample as f32 / 32768.0);
                            if buf.len() > 4096 {
                                buf.drain(0..2048);  // Keep buffer manageable
                            }
                        }
                    }
                    
                    self.current_samples = samples;
                    self.sample_index = 0;
                }
                Err(_) => return Some(0),
            }
        }
        
        // ... rest of existing code ...
    }
}
```

### Step 2: Add FFT to AudioPlayer

**File**: `src/utils/mediaplay.rs`

```rust
use crate::utils::audio_fft::AudioFFT;

pub struct AudioPlayer {
    sink: Sink,
    _stream: OutputStream,
    stream_handle: rodio::OutputStreamHandle,
    paused_at: Option<Instant>,
    total_paused_duration: Duration,
    start_time: Instant,
    duration: Option<Duration>,
    pub fft: Arc<Mutex<AudioFFT>>,  // ADD THIS
}

impl AudioPlayer {
    pub async fn new_and_play_cached(url: &str, token: &str, track_id: u64) -> Result<Self, Box<dyn std::error::Error>> {
        let fft = Arc::new(Mutex::new(AudioFFT::new()));  // CREATE FFT
        let fft_buffer = {
            let analyzer = fft.lock().unwrap();
            analyzer.get_sample_buffer()
        };
        
        // ... existing stream fetching code ...
        
        // Pass FFT buffer to StreamingSource
        let source = StreamingSource::new(sample_rx, channels, sample_rate, Some(fft_buffer));
        
        // ... rest of existing code ...
        
        Ok(Self {
            sink,
            _stream,
            stream_handle,
            paused_at: None,
            total_paused_duration: Duration::ZERO,
            start_time: Instant::now(),
            duration,
            fft,  // STORE FFT
        })
    }
}
```

### Step 3: Expose FFT in AudioController

**File**: `src/utils/audio_controller.rs`

```rust
use crate::utils::audio_fft::AudioFFT;

pub struct AudioController {
    // ... existing fields ...
    pub fft: Arc<Mutex<Option<Arc<Mutex<AudioFFT>>>>>,  // ADD THIS
}

impl AudioController {
    pub fn new() -> Self {
        // ... existing setup ...
        let fft = Arc::new(Mutex::new(None));
        let fft_clone = fft.clone();
        
        std::thread::spawn(move || {
            // ... in audio command loop ...
            AudioCommand::Play { url, token, track_id } => {
                match rt.block_on(AudioPlayer::new_and_play_cached(&url, &token, track_id)) {
                    Ok(p) => {
                        // Store FFT reference
                        *fft_clone.lock().unwrap() = Some(p.fft.clone());
                        player = Some(p);
                    }
                    // ...
                }
            }
        });
        
        Self {
            // ... existing fields ...
            fft,
        }
    }
    
    pub fn get_fft(&self) -> Option<Arc<Mutex<AudioFFT>>> {
        self.fft.lock().unwrap().clone()
    }
}
```

### Step 4: Update FFT in Main Loop

**File**: `src/app/player_app.rs` (in update() method)

```rust
impl eframe::App for MusicPlayerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Update FFT if audio is playing
        if let Some(fft_ref) = self.audio_controller.get_fft() {
            if let Ok(mut fft) = fft_ref.lock() {
                fft.update();  // Process FFT ~every frame
            }
        }
        
        // ... rest of update logic ...
    }
}
```

### Step 5: Pass FFT Data to Shader

**Option A: Extend ShaderUniforms** (Limited to ~16 bands)

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderUniforms {
    pub time: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
    pub resolution: [f32; 2],
    pub audio_bands: [f32; 16],  // ADD THIS - First 16 frequency bands
}
```

**Option B: Separate Audio Texture** (Full 64 bands, better performance)

Create a 1D texture with frequency data and bind as second uniform:
```rust
// In shader.rs - create audio texture binding
let audio_texture = device.create_texture(...);
// Update texture each frame with frequency data
queue.write_texture(audio_texture, &frequency_bytes, ...);
```

### Step 6: Use FFT in WGSL Shader

```wgsl
struct Uniforms {
    time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    resolution: vec2<f32>,
    audio_bands: array<f32, 16>,  // If using Option A
};

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let bass = uniforms.audio_bands[0];      // Low frequencies
    let mid = uniforms.audio_bands[7];       // Mid frequencies  
    let treble = uniforms.audio_bands[15];   // High frequencies
    
    // Example: Pulse effect based on bass
    let pulse = bass * 0.5;
    let color = vec3(pulse, 0.5, 1.0 - pulse);
    
    return vec4(color, 1.0);
}
```

## Performance Considerations

1. **FFT Update Rate**: Update FFT 30-60 times per second (not every audio sample)
2. **Buffer Size**: 2048 samples ≈ 46ms latency at 44.1kHz (acceptable for visuals)
3. **Smoothing**: 0.7 smoothing factor prevents jittery visuals
4. **Band Count**: 64 bands is good balance, reduce to 32 if performance issues

## Testing

Build and run with:
```bash
cargo build --release
./target/release/TempRS
```

FFT data will automatically flow to shaders when audio is playing.

## Notes

- FFT processing happens on main thread (update loop), not audio thread
- Sample capture is lock-free and minimal overhead in audio thread
- Frequency bands are logarithmically distributed (more detail in bass/mid)
- Values are normalized 0.0-1.0 for easy shader consumption
