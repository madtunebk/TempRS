// "Nebula Drift - Dynamic Cosmic Flow"
// Author: Master Of CP (assisted by Guardian Of Debug)
// Date: September 2024
// Special thanks to: Matteo Basei (Fractal Noise) and Inigo Quilez (Procedural Techniques)
// Ported to WGSL for TempRS

struct Uniforms {
    time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    resolution: vec2<f32>,
}
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

const OCTAVES: i32 = 4;

fn random(point: vec2<f32>) -> f32 {
    return fract(100.0 * sin(point.x + fract(100.0 * sin(point.y))));
}

fn noise(st: vec2<f32>) -> f32 {
    let i = floor(st);
    let f = fract(st);

    let a = random(i);
    let b = random(i + vec2<f32>(1.0, 0.0));
    let c = random(i + vec2<f32>(0.0, 1.0));
    let d = random(i + vec2<f32>(1.0, 1.0));

    let u = f * f * (3.0 - 2.0 * f);

    return mix(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
}

fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var freq = 1.0;
    var amp = 0.5;

    for (var i = 0; i < OCTAVES; i++) {
        value += amp * (noise((p - vec2<f32>(1.0)) * freq));
        freq *= 1.9;
        amp *= 0.6;
    }

    return value;
}

fn pattern(p: vec2<f32>) -> f32 {
    let aPos = vec2<f32>(sin(uniforms.time * 0.05), sin(uniforms.time * 0.1)) * 6.0;
    let aScale = vec2<f32>(3.0);
    let a = fbm(p * aScale + aPos);

    let bPos = vec2<f32>(sin(uniforms.time * 0.1), sin(uniforms.time * 0.1)) * 1.0;
    let bScale = vec2<f32>(0.5);
    let b = fbm((p + a) * bScale + bPos);

    let cPos = vec2<f32>(-0.6, -0.5) + vec2<f32>(sin(-uniforms.time * 0.01), sin(uniforms.time * 0.1)) * 2.0;
    let cScale = vec2<f32>(2.0);
    let c = fbm((p + b) * cScale + cPos);

    return c;
}

fn palette(t: f32) -> vec3<f32> {
    let a = vec3<f32>(0.5, 0.5, 0.5);
    let b = vec3<f32>(0.45, 0.25, 0.14);
    let c = vec3<f32>(1.0, 1.0, 1.0);
    let d = vec3<f32>(0.0, 0.1, 0.2);

    return a + b * cos(6.28318 * (c * t + d));
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VSOut {
    var out: VSOut;

    // Generate 2 triangles to cover the screen (6 vertices)
    // Triangle 1: (0,0), (1,0), (0,1)
    // Triangle 2: (1,0), (1,1), (0,1)
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),  // Bottom-left
        vec2<f32>(1.0, -1.0),   // Bottom-right
        vec2<f32>(-1.0, 1.0),   // Top-left
        vec2<f32>(1.0, -1.0),   // Bottom-right
        vec2<f32>(1.0, 1.0),    // Top-right
        vec2<f32>(-1.0, 1.0)    // Top-left
    );
    
    let p = pos[idx];
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv = (p * 0.5 + vec2<f32>(0.5, 0.5)) * uniforms.resolution / uniforms.resolution.y;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // Optimized nebula shader with reduced complexity
    
    // Calculate the UV coordinates for the scene
    var uv = in.uv * 2.0 - 1.0;
    uv.x *= uniforms.resolution.x / uniforms.resolution.y;

    // Oscillate the zoom factor using a sine wave
    let zoomSpeed = 0.5;
    let zoomAmount = 0.5;
    let zoomOffset = 1.0;
    
    // Calculate zoom factor
    var zoomFactor = zoomOffset + sin(uniforms.time * zoomSpeed) * zoomAmount;
    
    // Clamp the zoom factor to limit zoom out
    zoomFactor = clamp(zoomFactor, 0.5, 2.5);

    // Apply zoom effect (commented out as in original)
    // uv /= zoomFactor;

    // Apply a moving effect by adding a small time-based offset
    uv += vec2<f32>(sin(uniforms.time * 0.1), cos(uniforms.time * 0.1)) * 0.02;

    // Random screen shake effect every 10-30 seconds
    let shakeInterval = 10.0 + random(vec2<f32>(floor(uniforms.time * 0.01))) * 20.0;
    let shakeStart = select(0.0, 1.0, (uniforms.time % shakeInterval) < 0.2);
    let shakeIntensity = 0.02 * shakeStart * sin(uniforms.time * 125.0);

    // Apply the shake effect to the UV coordinates
    uv += vec2<f32>(
        random(uv + uniforms.time) - 0.5, 
        random(uv + uniforms.time * 2.0) - 0.5
    ) * shakeIntensity;

    // Apply time-based changes to make the image dynamic
    let value = pow(pattern(uv), 2.0);
    var color = palette(value);

    // Apply gamma correction
    let gamma = 0.5;
    color = pow(color, vec3<f32>(1.0 / gamma));

    // Reduce brightness by scaling color
    let brightnessFactor = 2.0;
    color *= brightnessFactor;

    // Output to screen
    return vec4<f32>(color, 1.0);
}

