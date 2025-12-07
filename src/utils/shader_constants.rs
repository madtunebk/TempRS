//! Centralized shader constants and boilerplate code
#![allow(dead_code)]
//!
//! All shader-related constants are defined here once to avoid duplication
//! across the codebase.

/// Default vertex shader code (standard full-screen triangle)
pub const DEFAULT_VERTEX: &str = r#"@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var out: VSOut;
    let x = f32((vi & 1u) << 2u);
    let y = f32((vi & 2u) << 1u);
    out.pos = vec4<f32>(x - 1.0, 1.0 - y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5, y * 0.5);
    return out;
}
"#;

/// Default fragment shader code (simple gradient for fallback)
pub const DEFAULT_FRAGMENT: &str = r#"@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.uv.x, in.uv.y, 0.5, 1.0);
}
"#;

/// Standard boilerplate auto-injected into every shader
///
/// Includes:
/// - Uniforms struct with time, audio bands, resolution, and color correction
/// - VSOut struct for vertex shader output
/// - Color correction helper functions
pub const SHADER_BOILERPLATE: &str = r#"
// Auto-injected uniforms (available in all shaders)
struct Uniforms {
    time: f32,
    audio_bass: f32,
    audio_mid: f32,
    audio_high: f32,
    resolution: vec2<f32>,
    gamma: f32,
    contrast: f32,
    saturation: f32,
    _pad0: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Auto-injected vertex output structure
struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// User-loaded image textures (iChannel0-3 - ShaderToy compatible)
@group(1) @binding(8)
var iChannel0: texture_2d<f32>;

@group(1) @binding(9)
var iChannel0Sampler: sampler;

@group(1) @binding(10)
var iChannel1: texture_2d<f32>;

@group(1) @binding(11)
var iChannel1Sampler: sampler;

@group(1) @binding(12)
var iChannel2: texture_2d<f32>;

@group(1) @binding(13)
var iChannel2Sampler: sampler;

@group(1) @binding(14)
var iChannel3: texture_2d<f32>;

@group(1) @binding(15)
var iChannel3Sampler: sampler;

// Color correction helper functions
fn applyGamma(color: vec3<f32>, gamma: f32) -> vec3<f32> {
    return pow(color, vec3<f32>(1.0 / gamma));
}

fn applyContrast(color: vec3<f32>, contrast: f32) -> vec3<f32> {
    // contrast: 0.5 = half contrast, 1.0 = normal, 2.0 = double contrast
    return ((color - 0.5) * contrast) + 0.5;
}

fn applySaturation(color: vec3<f32>, saturation: f32) -> vec3<f32> {
    // saturation: 0.0 = grayscale, 1.0 = normal, 2.0 = hyper saturated
    let gray = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    return mix(vec3<f32>(gray), color, saturation);
}

fn applyColorCorrection(color: vec3<f32>, gamma: f32, contrast: f32, saturation: f32) -> vec3<f32> {
    var result = color;
    result = applyContrast(result, contrast);
    result = applySaturation(result, saturation);
    result = applyGamma(result, gamma);
    return clamp(result, vec3<f32>(0.0), vec3<f32>(1.0));
}
"#;

/// Standard vertex shader auto-injected if user doesn't provide one
///
/// Generates a full-screen triangle using vertex index only
pub const STANDARD_VERTEX: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var out: VSOut;
    let x = f32((vi & 1u) << 2u);
    let y = f32((vi & 2u) << 1u);
    out.pos = vec4<f32>(x - 1.0, 1.0 - y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5, y * 0.5);
    return out;
}
"#;

/// Multi-pass texture bindings (added for MainImage and buffers that need texture access)
/// Layout matches multi_buffer_pipeline.rs bind group layout:
/// Buffer A: texture @0, sampler @1
/// Buffer B: texture @2, sampler @3
/// Buffer C: texture @4, sampler @5
/// Buffer D: texture @6, sampler @7
pub const TEXTURE_BINDINGS: &str = r#"
// Multi-pass texture bindings
@group(1) @binding(0) var buffer_a_texture: texture_2d<f32>;
@group(1) @binding(1) var buffer_a_sampler: sampler;
@group(1) @binding(2) var buffer_b_texture: texture_2d<f32>;
@group(1) @binding(3) var buffer_b_sampler: sampler;
@group(1) @binding(4) var buffer_c_texture: texture_2d<f32>;
@group(1) @binding(5) var buffer_c_sampler: sampler;
@group(1) @binding(6) var buffer_d_texture: texture_2d<f32>;
@group(1) @binding(7) var buffer_d_sampler: sampler;
"#;

/// Default font size for the shader editor
pub const DEFAULT_FONT_SIZE: f32 = 14.0;

/// Minimum font size for the shader editor
pub const MIN_FONT_SIZE: f32 = 8.0;

/// Maximum font size for the shader editor
pub const MAX_FONT_SIZE: f32 = 32.0;

/// Default buffer resolution (width, height)
pub const DEFAULT_BUFFER_RESOLUTION: [u32; 2] = [1920, 1080];
