// RGBA Demo - Main Image: COMBINE ALL CHANNELS
// Samples all 4 buffers and combines them with color swapping

struct Uniforms {
    time: f32,
    audio_bass: f32,
    audio_mid: f32,
    audio_high: f32,
    resolution: vec2<f32>,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

// Texture inputs from all buffers
@group(1) @binding(0) var bufferA: texture_2d<f32>;
@group(1) @binding(1) var samplerA: sampler;
@group(1) @binding(2) var bufferB: texture_2d<f32>;
@group(1) @binding(3) var samplerB: sampler;
@group(1) @binding(4) var bufferC: texture_2d<f32>;
@group(1) @binding(5) var samplerC: sampler;
@group(1) @binding(6) var bufferD: texture_2d<f32>;
@group(1) @binding(7) var samplerD: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    
    // Sample all buffers (each returns grayscale representing their channel)
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
    
    // Add some overall brightness boost
    color = pow(color, vec3<f32>(0.8));
    
    return vec4<f32>(color, 1.0);
}
