# Shadertoy to WGSL Converter

Automated tool to convert GLSL shaders from Shadertoy.com to WGSL format for TempRS.

## Quick Start

```bash
# 1. Copy Shadertoy shader code and save as .glsl file
# 2. Run converter
./target/release/shadertoy_converter shader.glsl

# Or specify output file
./target/release/shadertoy_converter shader.glsl output.wgsl

# Or use full paths
./target/release/shadertoy_converter ideas/fire.glsl src/shaders/fire.wgsl
```

## Usage Examples

```bash
# Convert with default output (src/shaders/converted_test.wgsl)
./target/release/shadertoy_converter ideas/example.glsl

# Convert with custom output
./target/release/shadertoy_converter ideas/plasma.glsl ideas/plasma_converted.wgsl

# Convert and integrate into app
./target/release/shadertoy_converter shader.glsl src/shaders/my_shader.wgsl
```

## Command Syntax

```
shadertoy_converter <input.glsl> [output.wgsl]
```

- `input.glsl` - Required. Path to GLSL shader file from Shadertoy
- `output.wgsl` - Optional. Output path (default: `src/shaders/converted_test.wgsl`)

## What It Auto-Converts

✅ **Function signatures**
- `void mainImage(out vec4 fragColor, in vec2 fragCoord)` → `@fragment fn fs_main(in: VSOut) -> @location(0) vec4<f32>`

✅ **Built-in variables**
- `fragCoord` → `in.uv * uniforms.resolution`
- `iTime` → `uniforms.time`
- `iResolution.xy` → `uniforms.resolution`
- `iResolution.x/y` → `uniforms.resolution.x/y`

✅ **Type conversions**
- `vec2/vec3/vec4` → `vec2<f32>/vec3<f32>/vec4<f32>`
- `mat2/mat3/mat4` → `mat2x2<f32>/mat3x3<f32>/mat4x4<f32>`
- `float` → `f32`

✅ **Output**
- `fragColor =` → `return`

✅ **Structure**
- Adds required `Uniforms` struct
- Adds `VSOut` struct
- Adds vertex shader `vs_main`
- Includes audio fields (bass, mid, high)

## What Needs Manual Fixing

⚠️ **Common issues to fix manually:**

### 1. Function Signatures (CRITICAL)
The converter is naive about functions. You'll need to fix:

**Before (GLSL):**
```glsl
float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}
```

**After converter (BROKEN):**
```wgsl
fn hash(vec2<f32> p) -> f32 -> f32 {  // DOUBLE RETURN TYPE!
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
```

**Fixed:**
```wgsl
fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
```

### 2. Type Annotations
WGSL requires parameter names before types:

**GLSL:**
```glsl
void myFunc(float x, vec2 uv)
```

**WGSL:**
```wgsl
fn myFunc(x: f32, uv: vec2<f32>)
```

### 3. Constants
**GLSL:**
```glsl
const float PI = 3.14159;
```

**WGSL:**
```wgsl
const PI: f32 = 3.14159;
```

### 4. mod() vs fmod()
GLSL `mod()` has different semantics than WGSL `fmod()`. Often you want:

**GLSL:**
```glsl
float x = mod(5.0, 3.0);  // Result: 2.0
```

**WGSL (Option 1 - fmod):**
```wgsl
let x = fmod(5.0, 3.0);  // Floating point modulo
```

**WGSL (Option 2 - custom):**
```wgsl
fn mod_f32(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}
```

### 5. Arrays
**GLSL:**
```glsl
float arr[3] = float[3](1.0, 2.0, 3.0);
```

**WGSL:**
```wgsl
var arr: array<f32, 3> = array<f32, 3>(1.0, 2.0, 3.0);
```

### 6. Texture Sampling
If shader uses `iChannel0`, `iChannel1`, etc., you need to:
1. Define texture bindings
2. Define samplers
3. Replace `texture(iChannel0, uv)` with `textureSample(tex0, samp0, uv)`

### 7. Double Type Issues
Converter can create `vec4<f32><f32>` - remove duplicate `<f32>`.

## Audio Reactivity

The converter adds audio fields to uniforms automatically:

```wgsl
struct Uniforms {
    time: f32,
    audio_bass: f32,   // Low frequencies (0-250Hz)
    audio_mid: f32,    // Mid frequencies (250-2000Hz)  
    audio_high: f32,   // High frequencies (2000Hz+)
    resolution: vec2<f32>,
    _pad0: vec2<f32>,
}
```

**Example usage in shader:**
```wgsl
// Make colors pulse with bass
let brightness = 1.0 + uniforms.audio_bass * 0.5;
let col = base_color * brightness;

// Sparkles from high frequencies
let sparkle_amount = uniforms.audio_high * 2.0;
```

## Testing Workflow

### Option 1: Shader Test Binary
```bash
# 1. Convert shader
./target/release/shadertoy_converter ideas/fire.glsl ideas/fire.wgsl

# 2. Edit shader_test.rs to load your shader
# Change: include_str!("../ideas/ambient_glow.wgsl")
# To:     include_str!("../ideas/fire.wgsl")

# 3. Build and run
cargo build --release --bin shader_test
RUST_LOG=info ./target/release/shader_test
```

### Option 2: Integrate into Main App
```bash
# 1. Convert shader
./target/release/shadertoy_converter shader.glsl src/shaders/my_shader.wgsl

# 2. Edit player_app.rs
# Change: include_str!("../shaders/universe_within.wgsl")
# To:     include_str!("../shaders/my_shader.wgsl")

# 3. Build and run
cargo build --release
./target/release/TempRS
```

## Example Conversion

### Input (Shadertoy GLSL)
```glsl
void mainImage( out vec4 fragColor, in vec2 fragCoord )
{
    vec2 uv = fragCoord/iResolution.xy;
    vec3 col = 0.5 + 0.5*cos(iTime+uv.xyx+vec3(0,2,4));
    fragColor = vec4(col,1.0);
}
```

### Output (WGSL - after manual fixes)
```wgsl
@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32>
{
    let uv = in.uv;  // Already normalized by VSOut
    let col = vec3<f32>(0.5) + vec3<f32>(0.5) * cos(uniforms.time + uv.xyx + vec3<f32>(0.0, 2.0, 4.0));
    return vec4<f32>(col, 1.0);
}
```

## Limitations

❌ **Won't work for:**
- Shaders using buffers (Buffer A, B, C, D)
- Shaders using textures/images (iChannel0-3)
- Cubemaps
- Sound input
- Keyboard input
- 3D textures
- Multiple render passes

✅ **Works best for:**
- Simple 2D procedural shaders
- Noise-based effects
- Mathematical patterns
- Single-pass effects

## Tips

1. **Start simple**: Try basic shaders first (plasma, noise, patterns)
2. **Check complexity**: Avoid shaders with buffers, textures, or 3D
3. **Test incrementally**: Fix one error at a time
4. **Use shader_test**: Faster iteration than full app rebuild
5. **Read errors carefully**: WGSL error messages are usually helpful
6. **Compare working shaders**: Look at `universe_within.wgsl` for reference

## Recommended Shadertoy Examples

**Easy to convert:**
- Simple plasma/noise patterns
- Mathematical visualizations
- 2D fractals (Mandelbrot, Julia)
- Color gradients

**Medium difficulty:**
- Raymarched 2D shapes
- Simple particle systems
- Wave simulations

**Hard/Impossible:**
- Volumetric rendering
- Multiple render passes
- Texture-dependent effects
- 3D raymarching with reflections

## File Structure

```
TempRS/
├── testunits/
│   └── shadertoy_converter.rs    # Converter tool
├── src/
│   └── shaders/
│       ├── universe_within.wgsl  # Reference shader
│       ├── plasma.wgsl           # Reference shader
│       └── converted_test.wgsl   # Converter output (auto-generated)
└── ideas/
    └── *.wgsl                    # Your experimental shaders
```

## Next Steps

After converting a shader:
1. Fix all compilation errors
2. Test with `shader_test` binary
3. Add audio reactivity if desired
4. Optimize for performance (30 FPS target)
5. Integrate into main app if successful
