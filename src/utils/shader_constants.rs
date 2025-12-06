//! Centralized shader constants and boilerplate code
//!
//! All shader-related constants are defined here once to avoid duplication
//! across the codebase.

/// Standard boilerplate auto-injected into every shader
///
/// Includes:
/// - Uniforms struct with time, audio bands, and resolution
/// - VSOut struct for vertex shader output
#[allow(dead_code)]
pub const SHADER_BOILERPLATE: &str = r#"
// Auto-injected uniforms (available in all shaders)
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

// Auto-injected vertex output structure
struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
"#;

/// Standard vertex shader auto-injected if user doesn't provide one
///
/// Generates a full-screen triangle using vertex index only
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub const DEFAULT_FONT_SIZE: f32 = 14.0;

/// Minimum font size for the shader editor
#[allow(dead_code)]
pub const MIN_FONT_SIZE: f32 = 8.0;

/// Maximum font size for the shader editor
#[allow(dead_code)]
pub const MAX_FONT_SIZE: f32 = 32.0;

/// Default buffer resolution (width, height)
#[allow(dead_code)]
pub const DEFAULT_BUFFER_RESOLUTION: [u32; 2] = [1920, 1080];
