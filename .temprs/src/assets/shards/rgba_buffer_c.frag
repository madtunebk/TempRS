// RGBA Demo - Buffer C: BLUE CHANNEL
// Animated circular spiral pattern

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
    
    // Center and create circular pattern
    let centered = (uv - 0.5) * vec2<f32>(aspect, 1.0);
    let dist = length(centered);
    let angle = atan2(centered.y, centered.x);
    
    // Rotating spiral
    let spiral = sin(dist * 15.0 - angle * 3.0 - u.time * 3.0) * 0.5 + 0.5;
    let high_pulse = u.audio_high * 0.3;
    
    // Blue channel intensity with audio reactivity
    let blue = spiral * (0.7 + high_pulse);
    
    // Output as grayscale (will be used as blue channel in MainImage)
    return vec4<f32>(blue, blue, blue, 1.0);
}
