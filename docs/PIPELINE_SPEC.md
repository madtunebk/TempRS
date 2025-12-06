# Multi-Pass Shader Pipeline Specification

This document defines the shared pipeline architecture between the shader editor and TempRS.

## Shader Format (.wgsls)

### File Structure

```wgsl
// Exported: YYYY-MM-DD HH:MM:SS

// BOILERPLATE
struct Uniforms {
    time: f32,
    audio_bass: f32,
    audio_mid: f32,
    audio_high: f32,
    resolution: vec2<f32>,
    _pad0: vec2<f32>,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// TEXTURE BINDINGS
@group(1) @binding(0) var buffer_a_texture: texture_2d<f32>;
@group(1) @binding(1) var buffer_a_sampler: sampler;
@group(1) @binding(2) var buffer_b_texture: texture_2d<f32>;
@group(1) @binding(3) var buffer_b_sampler: sampler;
@group(1) @binding(4) var buffer_c_texture: texture_2d<f32>;
@group(1) @binding(5) var buffer_c_sampler: sampler;
@group(1) @binding(6) var buffer_d_texture: texture_2d<f32>;
@group(1) @binding(7) var buffer_d_sampler: sampler;

// VERTEX
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VSOut { ... }

// MAINIMAGE
@fragment
fn fs_main_image(in: VSOut) -> @location(0) vec4<f32> { ... }

// BUFFERA
@fragment
fn fs_buffer_a(in: VSOut) -> @location(0) vec4<f32> { ... }

// BUFFERB
@fragment
fn fs_buffer_b(in: VSOut) -> @location(0) vec4<f32> { ... }

// BUFFERC
@fragment
fn fs_buffer_c(in: VSOut) -> @location(0) vec4<f32> { ... }

// BUFFERD
@fragment
fn fs_buffer_d(in: VSOut) -> @location(0) vec4<f32> { ... }
```

## Rendering Pipeline

### Render Order

1. **Buffer A** → Render with `fs_buffer_a` → Output to `texture_a`
2. **Buffer B** → Render with `fs_buffer_b` → Output to `texture_b`
3. **Buffer C** → Render with `fs_buffer_c` → Output to `texture_c`
4. **Buffer D** → Render with `fs_buffer_d` → Output to `texture_d`
5. **MainImage** → Render with `fs_main_image` (using textures A-D) → Output to screen

### Bind Group Layout

**Group 0: Uniforms** (All shaders)
- Binding 0: Uniform buffer (ShaderUniforms struct)
- Visibility: VERTEX | FRAGMENT

**Group 1: Textures** (MainImage only)
- Binding 0: buffer_a_texture (texture_2d)
- Binding 1: buffer_a_sampler (sampler)
- Binding 2: buffer_b_texture (texture_2d)
- Binding 3: buffer_b_sampler (sampler)
- Binding 4: buffer_c_texture (texture_2d)
- Binding 5: buffer_c_sampler (sampler)
- Binding 6: buffer_d_texture (texture_2d)
- Binding 7: buffer_d_sampler (sampler)

### Texture Configuration

**Resolution:** 1920x1080 (or window size)
**Format:** Rgba8Unorm
**Usage:** RENDER_ATTACHMENT | TEXTURE_BINDING
**Sampler:** Linear filtering, Clamp to edge

## Implementation in TempRS

### Required Changes

1. **Parse multiple entry points** from shader source
2. **Create 5 render pipelines:**
   - 1 for each buffer (Buffer A-D)
   - 1 for MainImage (with texture bindings)
3. **Create 4 offscreen textures** (1920x1080)
4. **Create texture bind group** for MainImage
5. **Render in sequence:** A → B → C → D → MainImage

### Code Reference

The shader editor's implementation is in:
- `src/utils/multi_buffer_pipeline.rs` - Full multi-pass pipeline
- `src/utils/shader_constants.rs` - Constants and bindings
- `src/screens/editor.rs` - Auto-injection logic

### Key Functions to Port

```rust
// From multi_buffer_pipeline.rs
pub struct MultiPassPipelines {
    uniform_buffer: Buffer,
    passes: HashMap<BufferKind, BufferPass>,
    // ...
}

impl MultiPassPipelines {
    pub fn new(device: &Device, format: TextureFormat, resolution: [u32; 2], sources: &HashMap<BufferKind, String>) -> Result<Self, ShaderError>
    pub fn render(&self, encoder: &mut CommandEncoder, screen_view: &TextureView, uniforms: &ShaderUniforms)
}
```

## Audio Integration

Uniforms are updated each frame:
- `uniforms.audio_bass` - Bass energy (0.0-1.0)
- `uniforms.audio_mid` - Mid energy (0.0-1.0)
- `uniforms.audio_high` - High energy (0.0-1.0)
- `uniforms.time` - Elapsed seconds
- `uniforms.resolution` - Screen/buffer size

## Graceful Degradation

If a buffer shader is missing or invalid:
- Skip that buffer pass
- Texture will be black/empty
- MainImage continues to render
- No crash or panic

## Testing

Test shader: `.TMRS/shaders/demo.wgsls`
- Should render multi-colored pattern with audio reactivity
- Each buffer contributes a color channel
- MainImage combines all with vignette and gamma
