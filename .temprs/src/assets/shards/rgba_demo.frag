// RGBA Multi-Buffer Demo
// Each buffer renders to a single color channel
// MainImage combines all channels with color swapping effects

// ========== UNIFORMS ==========
struct Uniforms {
    time: f32,
    audio_bass: f32,
    audio_mid: f32,
    audio_high: f32,
    resolution: vec2<f32>,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

// ========== TEXTURE INPUTS ==========
@group(1) @binding(0) var bufferA: texture_2d<f32>;
@group(1) @binding(1) var samplerA: sampler;
@group(1) @binding(2) var bufferB: texture_2d<f32>;
@group(1) @binding(3) var samplerB: sampler;
@group(1) @binding(4) var bufferC: texture_2d<f32>;
@group(1) @binding(5) var samplerC: sampler;
@group(1) @binding(6) var bufferD: texture_2d<f32>;
@group(1) @binding(7) var samplerD: sampler;

// ========== VERTEX OUTPUT ==========
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// ========== SHARED VERTEX SHADER ==========
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((vi & 1u) << 2u);
    let y = f32((vi & 2u) << 1u);
    out.position = vec4<f32>(x - 1.0, 1.0 - y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5, y * 0.5);
    return out;
}

// ========== BUFFER A: RED CHANNEL ==========
// Animated horizontal waves
@fragment
fn fs_buffer_a(in: VertexOutput) -> @location(0) vec4<f32> {
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

// ========== BUFFER B: GREEN CHANNEL ==========
// Animated vertical waves
@fragment
fn fs_buffer_b(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    
    // Create vertical waves
    let wave = sin(uv.x * 20.0 - u.time * 2.0) * 0.5 + 0.5;
    let mid_pulse = u.audio_mid * 0.3;
    
    // Green channel intensity with audio reactivity
    let green = wave * (0.7 + mid_pulse);
    
    // Output as grayscale (will be used as green channel in MainImage)
    return vec4<f32>(green, green, green, 1.0);
}

// ========== BUFFER C: BLUE CHANNEL ==========
// Animated circular pattern
@fragment
fn fs_buffer_c(in: VertexOutput) -> @location(0) vec4<f32> {
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

// ========== BUFFER D: ALPHA CHANNEL ==========
// Animated grid pattern
@fragment
fn fs_buffer_d(in: VertexOutput) -> @location(0) vec4<f32> {
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

// ========== MAIN IMAGE: COMBINE ALL CHANNELS ==========
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    
    // Sample all buffers
    let red_channel = textureSample(bufferA, samplerA, uv).r;
    let green_channel = textureSample(bufferB, samplerB, uv).r;
    let blue_channel = textureSample(bufferC, samplerC, uv).r;
    let alpha_mod = textureSample(bufferD, samplerD, uv).r;
    
    // Color swapping effect based on time
    let swap = (sin(u.time * 0.5) + 1.0) * 0.5; // 0 to 1
    
    var color: vec3<f32>;
    if swap < 0.25 {
        // Normal RGB
        color = vec3<f32>(red_channel, green_channel, blue_channel);
    } else if swap < 0.5 {
        // Swap R and G
        color = vec3<f32>(green_channel, red_channel, blue_channel);
    } else if swap < 0.75 {
        // Swap G and B
        color = vec3<f32>(red_channel, blue_channel, green_channel);
    } else {
        // Swap R and B
        color = vec3<f32>(blue_channel, green_channel, red_channel);
    }
    
    // Apply alpha modulation for intensity variation
    color *= alpha_mod;
    
    // Add some overall brightness
    color = pow(color, vec3<f32>(0.8));
    
    return vec4<f32>(color, 1.0);
}
