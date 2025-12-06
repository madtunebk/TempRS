// RGBA Demo - Buffer D: ALPHA/INTENSITY MODULATOR
// Animated grid pattern

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
    
    // Create grid
    let grid_x = sin(uv.x * 40.0 + u.time) * 0.5 + 0.5;
    let grid_y = sin(uv.y * 40.0 - u.time) * 0.5 + 0.5;
    let grid = grid_x * grid_y;
    
    // Alpha channel intensity
    let alpha = grid * 0.8 + 0.2; // Keep some minimum alpha
    
    // Output as grayscale (will be used as alpha/intensity modulator)
    return vec4<f32>(alpha, alpha, alpha, 1.0);
}
