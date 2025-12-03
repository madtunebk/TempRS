// Plasma shader - Ported from Shadertoy
// Original: https://www.shadertoy.com/view/llK3Dy
// Smooth plasma effect with flowing colors

struct Uniforms {
    time: f32,
    audio_bass: f32,
    audio_mid: f32,
    audio_high: f32,
    resolution: vec2<f32>,
    _pad0: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VSOut {
    var out: VSOut;
    let x = f32((idx & 1u) << 2u);
    let y = f32((idx & 2u) << 1u);
    out.pos = vec4<f32>(x - 1.0, 1.0 - y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5, y * 0.5);
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let uv = in.uv * uniforms.resolution;
    let p = (uv * 2.0 - uniforms.resolution) / min(uniforms.resolution.x, uniforms.resolution.y);
    
    // Audio-reactive time (bass controls speed)
    let t = uniforms.time * (0.5 + uniforms.audio_bass * 0.5);
    
    // Multiple plasma layers
    var v = 0.0;
    
    // Layer 1: Circular waves (bass reactive)
    v += sin(length(p) * (10.0 + uniforms.audio_bass * 5.0) - t * 2.0);
    
    // Layer 2: Diagonal waves (mid reactive)
    v += sin((p.x + p.y) * (8.0 + uniforms.audio_mid * 4.0) + t);
    
    // Layer 3: Rotating waves (high reactive)
    let angle = atan2(p.y, p.x);
    v += sin(angle * (5.0 + uniforms.audio_high * 3.0) + t * 1.5);
    
    // Layer 4: Distance-based oscillation
    v += sin(length(p * vec2<f32>(sin(t * 0.3), cos(t * 0.5))) * 8.0);
    
    // Normalize
    v = v * 0.25 + 0.5;
    
    // Colorful plasma gradient
    let r = sin(v * 6.28318 + 0.0) * 0.5 + 0.5;
    let g = sin(v * 6.28318 + 2.09439) * 0.5 + 0.5;  // +120 degrees
    let b = sin(v * 6.28318 + 4.18879) * 0.5 + 0.5;  // +240 degrees
    
    // Audio-reactive brightness (overall music level)
    let audio_level = (uniforms.audio_bass + uniforms.audio_mid + uniforms.audio_high) / 3.0;
    let brightness = 0.7 + 0.3 * (sin(t * 0.5) + audio_level);
    
    return vec4<f32>(r * brightness, g * brightness, b * brightness, 1.0);
}
