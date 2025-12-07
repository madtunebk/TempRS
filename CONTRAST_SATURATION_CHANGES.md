# Adding Contrast and Saturation to TempRS

## Changes Required

### 1. Update `src/utils/pipeline.rs` - ShaderUniforms

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderUniforms {
    pub time: f32,
    pub audio_bass: f32,
    pub audio_mid: f32,
    pub audio_high: f32,
    pub resolution: [f32; 2],
    pub gamma: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub _pad0: f32,  // Padding for 16-byte alignment (10 floats → need 2 more for 12)
    pub _pad1: f32,
    pub _pad2: f32,
}
```

**Update both ShaderCallback and MultiPassCallback prepare() methods to include:**
```rust
let gamma = *self.gamma.lock().unwrap();
let contrast = *self.contrast.lock().unwrap();
let saturation = *self.saturation.lock().unwrap();

let uniforms = ShaderUniforms {
    time: elapsed,
    audio_bass: bass,
    audio_mid: mid,
    audio_high: high,
    resolution,
    gamma,
    contrast,
    saturation,
    _pad0: 0.0,
    _pad1: 0.0,
    _pad2: 0.0,
};
```

**Add fields to ShaderCallback and MultiPassCallback structs:**
```rust
pub struct ShaderCallback {
    pub shader: Arc<ShaderPipeline>,
    pub bass_energy: Arc<std::sync::Mutex<f32>>,
    pub mid_energy: Arc<std::sync::Mutex<f32>>,
    pub high_energy: Arc<std::sync::Mutex<f32>>,
    pub gamma: Arc<std::sync::Mutex<f32>>,
    pub contrast: Arc<std::sync::Mutex<f32>>,
    pub saturation: Arc<std::sync::Mutex<f32>>,
}
```

### 2. Update `src/utils/shader_constants.rs` - SHADER_BOILERPLATE

```rust
pub const SHADER_BOILERPLATE: &str = r#"
// Auto-injected uniforms (available in all shaders)
struct Uniforms {
    time: f32,
    audio_bass: f32,
    audio_mid: f32,
    audio_high: f32,
    resolution: vec2<f32>,
    gamma: f32,
    contrast: f32,
    saturation: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Auto-injected vertex output structure
struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// User-loaded image textures (iChannel0-3 - ShaderToy compatible)
@group(1) @binding(8)
var iChannel0: texture_2d<f32>;

@group(1) @binding(9)
var iChannel0Sampler: sampler;

@group(1) @binding(10)
var iChannel1: texture_2d<f32>;

@group(1) @binding(11)
var iChannel1Sampler: sampler;

@group(1) @binding(12)
var iChannel2: texture_2d<f32>;

@group(1) @binding(13)
var iChannel2Sampler: sampler;

@group(1) @binding(14)
var iChannel3: texture_2d<f32>;

@group(1) @binding(15)
var iChannel3Sampler: sampler;

// Color correction helper functions
fn applyGamma(color: vec3<f32>, gamma: f32) -> vec3<f32> {
    return pow(color, vec3<f32>(1.0 / gamma));
}

fn applyContrast(color: vec3<f32>, contrast: f32) -> vec3<f32> {
    // contrast: 0.5 = half contrast, 1.0 = normal, 2.0 = double contrast
    return ((color - 0.5) * contrast) + 0.5;
}

fn applySaturation(color: vec3<f32>, saturation: f32) -> vec3<f32> {
    // saturation: 0.0 = grayscale, 1.0 = normal, 2.0 = hyper saturated
    let gray = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    return mix(vec3<f32>(gray), color, saturation);
}

fn applyColorCorrection(color: vec3<f32>, gamma: f32, contrast: f32, saturation: f32) -> vec3<f32> {
    var result = color;
    result = applyContrast(result, contrast);
    result = applySaturation(result, saturation);
    result = applyGamma(result, gamma);
    return clamp(result, vec3<f32>(0.0), vec3<f32>(1.0));
}
"#;
```

### 3. Update `src/app/shader_manager.rs`

**Add fields:**
```rust
pub struct ShaderManager {
    // Shader pipelines
    splash_shader: Option<Arc<ShaderPipeline>>,
    multi_pass_shader: Option<Arc<MultiPassPipelines>>,
    track_metadata_shader: Option<Arc<ShaderPipeline>>,
    
    // Color correction values
    pub gamma: Arc<Mutex<f32>>,
    pub contrast: Arc<Mutex<f32>>,
    pub saturation: Arc<Mutex<f32>>,
    
    // Hot-reload state
    shader_checksum: Option<String>,
    last_hot_reload_check: Instant,
    
    // WGPU resources (cached from creation context)
    wgpu_device: Option<Arc<Device>>,
    wgpu_queue: Option<Arc<Queue>>,
    wgpu_format: Option<TextureFormat>,
}
```

**Initialize in new():**
```rust
gamma: Arc::new(Mutex::new(1.0)),      // Default: no gamma correction
contrast: Arc::new(Mutex::new(1.0)),   // Default: normal contrast
saturation: Arc::new(Mutex::new(1.0)), // Default: normal saturation
```

**Load from JSON in load_from_json():**
```rust
// Load color correction values from JSON
if let Some(gamma_value) = shader_json.gamma {
    *self.gamma.lock().unwrap() = gamma_value;
    log::info!("[ShaderManager] Loaded gamma: {}", gamma_value);
}
if let Some(contrast_value) = shader_json.contrast {
    *self.contrast.lock().unwrap() = contrast_value;
    log::info!("[ShaderManager] Loaded contrast: {}", contrast_value);
}
if let Some(saturation_value) = shader_json.saturation {
    *self.saturation.lock().unwrap() = saturation_value;
    log::info!("[ShaderManager] Loaded saturation: {}", saturation_value);
}
```

**Add getter methods:**
```rust
pub fn contrast(&self) -> Arc<Mutex<f32>> {
    Arc::clone(&self.contrast)
}

pub fn saturation(&self) -> Arc<Mutex<f32>> {
    Arc::clone(&self.saturation)
}
```

### 4. Update `src/utils/shader_json.rs` - ShaderJson

**Add fields:**
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShaderJson {
    pub version: Option<String>,
    pub encoding: Option<String>,
    pub vertex: Option<String>,
    pub fragment: String,
    pub buffer_a: Option<String>,
    pub buffer_b: Option<String>,
    pub buffer_c: Option<String>,
    pub buffer_d: Option<String>,
    pub ichannel0: Option<String>,
    pub ichannel1: Option<String>,
    pub ichannel2: Option<String>,
    pub ichannel3: Option<String>,
    pub gamma: Option<f32>,
    pub contrast: Option<f32>,
    pub saturation: Option<f32>,
}
```

### 5. Update `src/screens/splash.rs` and `src/screens/now_playing.rs`

**Pass values to callbacks:**
```rust
// In splash.rs
ShaderCallback {
    shader: shader.clone(),
    bass_energy: app.bass_energy.clone(),
    mid_energy: app.mid_energy.clone(),
    high_energy: app.high_energy.clone(),
    gamma: app.shader_manager.gamma(),
    contrast: app.shader_manager.contrast(),
    saturation: app.shader_manager.saturation(),
}

// In now_playing.rs - both ShaderCallback and MultiPassCallback
MultiPassCallback {
    shader: multi_shader.clone(),
    bass_energy: app.bass_energy.clone(),
    mid_energy: app.mid_energy.clone(),
    high_energy: app.high_energy.clone(),
    gamma: app.shader_manager.gamma(),
    contrast: app.shader_manager.contrast(),
    saturation: app.shader_manager.saturation(),
}
```

### 6. Update `src/utils/multi_buffer_pipeline.rs`

Same as pipeline.rs - add fields to MultiPassCallback struct and use them in prepare().

## Testing

After applying changes:

1. Build: `cd ~/Projects/TempRS && cargo build`
2. Create test JSON with color correction:
```json
{
  "fragment": "...",
  "gamma": 2.2,
  "contrast": 1.2,
  "saturation": 1.1
}
```
3. Load and verify values are applied

## Default Values

- **Gamma**: 1.0 (no correction)
- **Contrast**: 1.0 (normal)
- **Saturation**: 1.0 (normal)

All values loaded from JSON or use defaults if not specified.
