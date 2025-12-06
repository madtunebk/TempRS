// RGBA Demo - Buffer B: GREEN CHANNEL
// Animated vertical waves

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
    
    // Create vertical waves
    let wave = sin(uv.x * 20.0 - u.time * 2.0) * 0.5 + 0.5;
    let mid_pulse = u.audio_mid * 0.3;
    
    // Green channel intensity with audio reactivity
    let green = wave * (0.7 + mid_pulse);
    
    // Output as grayscale (will be used as green channel in MainImage)
    return vec4<f32>(green, green, green, 1.0);
}
