#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::utils::BufferKind;
use crate::utils::shader_constants::{SHADER_BOILERPLATE, STANDARD_VERTEX, TEXTURE_BINDINGS};

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

    /// Base64-encoded PNG/JPEG image data for iChannel0-3
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ichannel0: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ichannel1: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ichannel2: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ichannel3: Option<String>,

    /// Gamma correction value (default: 1.0 = no correction)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamma: Option<f32>,

    /// Contrast adjustment (default: 1.0 = normal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contrast: Option<f32>,

    /// Saturation adjustment (default: 1.0 = normal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saturation: Option<f32>,
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
    pub fn encode_to_base64(code: &str) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, code.as_bytes())
    }

    /// Decode embedded base64 images to raw PNG/JPEG bytes
    /// Returns array of Option<Vec<u8>> for iChannel0-3
    pub fn decode_embedded_images(&self) -> [Option<Vec<u8>>; 4] {
        let mut images = [None, None, None, None];
        
        let channels = [&self.ichannel0, &self.ichannel1, &self.ichannel2, &self.ichannel3];
        
        for (i, channel) in channels.iter().enumerate() {
            if let Some(base64_data) = channel {
                match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data) {
                    Ok(bytes) => {
                        log::info!("Decoded embedded image for iChannel{} ({} bytes)", i, bytes.len());
                        images[i] = Some(bytes);
                    }
                    Err(e) => {
                        log::warn!("Failed to decode base64 for iChannel{}: {}", i, e);
                    }
                }
            }
        }
        
        images
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

        // Use centralized boilerplate from shader_constants (includes iChannel0-3)
        let boilerplate = SHADER_BOILERPLATE;

        // Vertex shader (use provided or standard default)
        let vertex_shader = self.vertex.as_deref()
            .unwrap_or(STANDARD_VERTEX);

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
        // If we have buffers, inject texture bindings for BufferA-D access
        let main_image_code = if has_buffers {
            format!("{}\n{}\n{}\n{}", boilerplate, TEXTURE_BINDINGS, vertex_shader, &self.fragment)
        } else {
            format!("{}\n{}\n{}", boilerplate, vertex_shader, &self.fragment)
        };
        map.insert(BufferKind::MainImage, main_image_code);

        map
    }
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
