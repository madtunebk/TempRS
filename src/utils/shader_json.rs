use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::utils::BufferKind;

/// JSON shader format for editor exports
/// Supports both plain text and base64-encoded shaders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderJson {
    #[serde(default = "default_version")]
    pub version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exported_at: Option<String>,

    /// Encoding format: "plain" or "base64" (default: "plain")
    #[serde(default = "default_encoding")]
    pub encoding: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex: Option<String>,

    pub fragment: String,  // MainImage - required

    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffer_a: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffer_b: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffer_c: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffer_d: Option<String>,
}

fn default_version() -> String {
    "1.0".to_string()
}

fn default_encoding() -> String {
    "plain".to_string()
}

impl ShaderJson {
    /// Parse JSON shader from string and decode if needed
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let mut shader: Self = serde_json::from_str(json_str)?;

        // Decode base64 fields if encoding is "base64"
        if shader.encoding == "base64" {
            shader.fragment = decode_base64(&shader.fragment).unwrap_or(shader.fragment);
            if let Some(ref vertex) = shader.vertex {
                shader.vertex = Some(decode_base64(vertex).unwrap_or_else(|| vertex.clone()));
            }
            if let Some(ref buffer_a) = shader.buffer_a {
                shader.buffer_a = Some(decode_base64(buffer_a).unwrap_or_else(|| buffer_a.clone()));
            }
            if let Some(ref buffer_b) = shader.buffer_b {
                shader.buffer_b = Some(decode_base64(buffer_b).unwrap_or_else(|| buffer_b.clone()));
            }
            if let Some(ref buffer_c) = shader.buffer_c {
                shader.buffer_c = Some(decode_base64(buffer_c).unwrap_or_else(|| buffer_c.clone()));
            }
            if let Some(ref buffer_d) = shader.buffer_d {
                shader.buffer_d = Some(decode_base64(buffer_d).unwrap_or_else(|| buffer_d.clone()));
            }
        }

        Ok(shader)
    }

    /// Encode shader code to base64 for safe JSON storage
    #[allow(dead_code)]
    pub fn encode_to_base64(code: &str) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, code.as_bytes())
    }

    /// Convert to HashMap for MultiPassPipelines
    /// Injects boilerplate (uniforms, VSOut, vertex shader, texture bindings)
    pub fn to_shader_map(&self) -> HashMap<BufferKind, String> {
        let mut map = HashMap::new();

        // Check if we have any buffers
        let has_buffers = self.buffer_a.is_some()
            || self.buffer_b.is_some()
            || self.buffer_c.is_some()
            || self.buffer_d.is_some();

        // Boilerplate (injected to ALL shaders)
        let boilerplate = generate_boilerplate();

        // Vertex shader (use provided or default)
        let vertex_shader = self.vertex.as_ref()
            .map(|v| v.clone())
            .unwrap_or_else(|| generate_default_vertex());

        // Process BufferA
        if let Some(buffer_a_code) = &self.buffer_a {
            let full_shader = format!("{}\n{}\n{}", boilerplate, vertex_shader, buffer_a_code);
            map.insert(BufferKind::BufferA, full_shader);
        }

        // Process BufferB
        if let Some(buffer_b_code) = &self.buffer_b {
            let full_shader = format!("{}\n{}\n{}", boilerplate, vertex_shader, buffer_b_code);
            map.insert(BufferKind::BufferB, full_shader);
        }

        // Process BufferC
        if let Some(buffer_c_code) = &self.buffer_c {
            let full_shader = format!("{}\n{}\n{}", boilerplate, vertex_shader, buffer_c_code);
            map.insert(BufferKind::BufferC, full_shader);
        }

        // Process BufferD
        if let Some(buffer_d_code) = &self.buffer_d {
            let full_shader = format!("{}\n{}\n{}", boilerplate, vertex_shader, buffer_d_code);
            map.insert(BufferKind::BufferD, full_shader);
        }

        // Process MainImage (fragment)
        // If we have buffers, inject texture bindings
        let main_image_code = if has_buffers {
            let texture_bindings = generate_texture_bindings();
            format!("{}\n{}\n{}\n{}", boilerplate, texture_bindings, vertex_shader, &self.fragment)
        } else {
            format!("{}\n{}\n{}", boilerplate, vertex_shader, &self.fragment)
        };
        map.insert(BufferKind::MainImage, main_image_code);

        map
    }
}

/// Generate uniforms + VSOut boilerplate (always injected)
fn generate_boilerplate() -> String {
    r#"// Auto-injected boilerplate
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

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
"#.to_string()
}

/// Generate default vertex shader
fn generate_default_vertex() -> String {
    r#"// Auto-injected default vertex shader
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var out: VSOut;
    let x = f32((vi & 1u) << 2u);
    let y = f32((vi & 2u) << 1u);
    out.pos = vec4<f32>(x - 1.0, 1.0 - y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5, y * 0.5);
    return out;
}
"#.to_string()
}

/// Generate texture bindings for multi-pass (only for MainImage)
fn generate_texture_bindings() -> String {
    r#"// Auto-injected texture bindings for multi-pass
@group(1) @binding(0) var buffer_a_texture: texture_2d<f32>;
@group(1) @binding(1) var buffer_a_sampler: sampler;
@group(1) @binding(2) var buffer_b_texture: texture_2d<f32>;
@group(1) @binding(3) var buffer_b_sampler: sampler;
@group(1) @binding(4) var buffer_c_texture: texture_2d<f32>;
@group(1) @binding(5) var buffer_c_sampler: sampler;
@group(1) @binding(6) var buffer_d_texture: texture_2d<f32>;
@group(1) @binding(7) var buffer_d_sampler: sampler;
"#.to_string()
}

/// Decode base64 string to UTF-8 text
fn decode_base64(encoded: &str) -> Option<String> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_shader() {
        let json = r#"{
            "version": "1.0",
            "fragment": "@fragment\nfn fs_main(in: VSOut) -> @location(0) vec4<f32> {\n    return vec4(in.uv.x, in.uv.y, 0.0, 1.0);\n}"
        }"#;

        let shader = ShaderJson::from_json(json).unwrap();
        let map = shader.to_shader_map();

        assert_eq!(map.len(), 1);
        assert!(map.contains_key(&BufferKind::MainImage));
        assert!(!map.get(&BufferKind::MainImage).unwrap().contains("buffer_a_texture"));
    }

    #[test]
    fn test_multipass_shader() {
        let json = r#"{
            "version": "1.0",
            "fragment": "MainImage code",
            "buffer_a": "BufferA code"
        }"#;

        let shader = ShaderJson::from_json(json).unwrap();
        let map = shader.to_shader_map();

        assert_eq!(map.len(), 2);
        assert!(map.contains_key(&BufferKind::MainImage));
        assert!(map.contains_key(&BufferKind::BufferA));
        assert!(map.get(&BufferKind::MainImage).unwrap().contains("buffer_a_texture"));
    }
}
