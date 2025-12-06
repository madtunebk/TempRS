# Shader JSON Format Specification

## Structure

```json
{
  "version": "1.0",
  "exported_at": "2025-12-06T02:26:42Z",
  "vertex": "optional vertex shader code",
  "fragment": "fragment shader code (required)",
  "buffer_a": "optional BufferA fragment shader",
  "buffer_b": "optional BufferB fragment shader",
  "buffer_c": "optional BufferC fragment shader",
  "buffer_d": "optional BufferD fragment shader"
}
```

## Rules

1. **Only `fragment` is required** - all other fields are optional
2. **Auto-injection happens in player:**
   - Uniforms struct (always injected)
   - VSOut struct (always injected)
   - Vertex shader (injected if not provided)
   - Texture bindings (only for MainImage if buffers exist)
3. **Entry points:**
   - All fragment shaders use `@fragment fn fs_main()`
   - Vertex shader uses `@vertex fn vs_main()`

## Examples

### Simple single-pass shader (fragment only)
```json
{
  "version": "1.0",
  "fragment": "@fragment\nfn fs_main(in: VSOut) -> @location(0) vec4<f32> {\n    return vec4(in.uv.x, in.uv.y, 0.0, 1.0);\n}"
}
```

### Multi-pass with all buffers
```json
{
  "version": "1.0",
  "fragment": "MainImage shader code...",
  "buffer_a": "BufferA shader code...",
  "buffer_b": "BufferB shader code...",
  "buffer_c": "BufferC shader code...",
  "buffer_d": "BufferD shader code..."
}
```

### Multi-pass with only BufferA and MainImage
```json
{
  "version": "1.0",
  "fragment": "MainImage shader code...",
  "buffer_a": "BufferA shader code..."
}
```

## Auto-Injection Details

### Uniforms (always injected to ALL shaders)
```wgsl
struct Uniforms {
    time: f32,
    audio_bass: f32,
    audio_mid: f32,
    audio_high: f32,
    resolution: vec2<f32>,
    _pad0: vec2<f32>,
}
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;
```

### VSOut (always injected to ALL shaders)
```wgsl
struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
```

### Default Vertex Shader (injected if `vertex` field is missing)
```wgsl
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var out: VSOut;
    let x = f32((vi & 1u) << 2u);
    let y = f32((vi & 2u) << 1u);
    out.pos = vec4<f32>(x - 1.0, 1.0 - y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5, y * 0.5);
    return out;
}
```

### Texture Bindings (ONLY injected to MainImage if any buffer exists)
```wgsl
@group(1) @binding(0) var buffer_a_texture: texture_2d<f32>;
@group(1) @binding(1) var buffer_a_sampler: sampler;
@group(1) @binding(2) var buffer_b_texture: texture_2d<f32>;
@group(1) @binding(3) var buffer_b_sampler: sampler;
@group(1) @binding(4) var buffer_c_texture: texture_2d<f32>;
@group(1) @binding(5) var buffer_c_sampler: sampler;
@group(1) @binding(6) var buffer_d_texture: texture_2d<f32>;
@group(1) @binding(7) var buffer_d_sampler: sampler;
```

Note: Texture bindings are injected to MainImage even if only some buffers exist. Unused textures will just sample black.
