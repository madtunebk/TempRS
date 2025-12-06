// RGBA Demo - Buffer A: RED CHANNEL
// Animated horizontal waves

struct Uniforms {
    time: f32,
    audio_bass: f32,
    audio_mid: f32,
    audio_high: f32,
    resolution: vec2<f32>,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let aspect = u.resolution.x / u.resolution.y;
    
    // Create horizontal waves
    let wave = sin(uv.y * 20.0 + u.time * 2.0) * 0.5 + 0.5;
    let bass_pulse = u.audio_bass * 0.3;
    
    // Red channel intensity with audio reactivity
    let red = wave * (0.7 + bass_pulse);
    
    // Output as grayscale (will be used as red channel in MainImage)
    return vec4<f32>(red, red, red, 1.0);
}
