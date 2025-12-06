// Multi-Pass Buffer B
// Placeholder shader - can be replaced with editor exports
// This buffer can be sampled by BufferC, BufferD, and MainImage

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

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var out: VSOut;
    let x = f32((vi & 1u) << 2u);
    let y = f32((vi & 2u) << 1u);
    out.pos = vec4<f32>(x - 1.0, 1.0 - y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5, y * 0.5);
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // Mid-frequency wave pattern
    let wave = sin(in.uv.x * 10.0 + uniforms.time * 2.0) * uniforms.audio_mid;
    let mid_boost = uniforms.audio_mid * 0.5 + 0.5;

    let col = vec3(
        in.uv.y * 0.5,
        wave * 0.5 + 0.5,
        mid_boost
    );

    return vec4(col, 1.0);
}
