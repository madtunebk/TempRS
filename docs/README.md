# TempRS Shader System

Audio-reactive WGSL shaders with multi-pass rendering support.

## Features

- **Multi-pass rendering**: 4 offscreen buffers (Buffer A-D) + MainImage compositor
- **Hot-reload**: Edit shader JSON → auto-reload every 2 seconds
- **Audio reactivity**: Real-time FFT data (bass, mid, high frequencies)
- **Editor integration**: Compatible with [wgsls_editor](https://github.com/madtunebk/wgsls_editor)
- **Validation**: Naga-based WGSL validation with helpful error messages
- **Auto-injection**: No need to write boilerplate (uniforms, vertex shader, texture bindings)

## Shader Location

TempRS loads shaders from:
- **Primary**: `~/.cache/TempRS/shaders/shader.json` (hot-reloadable)
- **Fallback**: `src/assets/shards/demo_multipass.json` (embedded)

## JSON Format

**Minimal single-pass:**
```json
{
  "version": "1.0",
  "fragment": "@fragment\nfn fs_main(in: VSOut) -> @location(0) vec4<f32> {\n    return vec4(in.uv.x, in.uv.y, 0.0, 1.0);\n}"
}
```

**Multi-pass with all buffers:**
```json
{
  "version": "1.0",
  "encoding": "base64",
  "fragment": "base64_encoded_mainimage",
  "buffer_a": "base64_encoded_buffer_a",
  "buffer_b": "base64_encoded_buffer_b",
  "buffer_c": "base64_encoded_buffer_c",
  "buffer_d": "base64_encoded_buffer_d"
}
```

## Auto-Injected Boilerplate

You only write fragment shader logic. TempRS automatically injects:

**Uniforms** (available in all shaders):
```wgsl
struct Uniforms {
    time: f32,
    audio_bass: f32,
    audio_mid: f32,
    audio_high: f32,
    resolution: vec2<f32>,
    _pad0: vec2<f32>,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
```

**Vertex Output**:
```wgsl
struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
```

**Vertex Shader** (if not provided):
```wgsl
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut { ... }
```

**Texture Bindings** (for MainImage if buffers exist):
```wgsl
@group(1) @binding(0) var buffer_a_texture: texture_2d<f32>;
@group(1) @binding(1) var buffer_a_sampler: sampler;
// ... (B, C, D textures and samplers)
```

## Pipeline Architecture

**Multi-pass render order:**
1. Buffer A → render `fs_main` → offscreen texture A
2. Buffer B → render `fs_main` → offscreen texture B
3. Buffer C → render `fs_main` → offscreen texture C
4. Buffer D → render `fs_main` → offscreen texture D
5. MainImage → render `fs_main` (samples textures A-D) → screen

**Each buffer shader:**
- Gets uniforms (time, audio data, resolution)
- Renders to offscreen texture (1920x1080 or window size)
- Uses entry point `@fragment fn fs_main(...)`

**MainImage shader:**
- Gets uniforms + texture bindings (if buffers exist)
- Samples buffer textures via `textureSample(buffer_a_texture, buffer_a_sampler, uv)`
- Renders final output to screen

## Usage

See [`SETUP.md`](SETUP.md) for shader editor integration guide.
See [`PIPELINE_SPEC.md`](PIPELINE_SPEC.md) for technical specification.
See [`../src/assets/shards/shader_format.md`](../src/assets/shards/shader_format.md) for JSON format details.