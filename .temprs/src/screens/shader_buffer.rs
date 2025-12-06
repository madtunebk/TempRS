//! Unified shader buffer data structure
//!
//! Replaces the previous separate MainImageTab, BufferATab, etc. with a single
//! generic structure that can represent any shader buffer (MainImage or Buffer A-D).

use crate::ui_components::shader_editor;
use crate::utils::BufferKind;
use eframe::egui;

/// A shader buffer containing vertex and fragment shader code
///
/// This structure is used for all buffer types (MainImage, Buffer A-D),
/// eliminating code duplication across 5 separate tab implementations.
pub struct ShaderBuffer {
    /// The type of buffer this represents
    pub kind: BufferKind,
    /// Fragment shader source code
    pub fragment_code: String,
    /// Vertex shader source code
    pub vertex_code: String,
}

impl ShaderBuffer {
    /// Create a new shader buffer
    pub fn new(kind: BufferKind, vertex: String, fragment: String) -> Self {
        Self {
            kind,
            fragment_code: fragment,
            vertex_code: vertex,
        }
    }

    /// Render the editor for this buffer
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `is_fragment_tab` - Whether to show fragment (true) or vertex (false) code
    /// * `font_size` - Font size for the editor
    pub fn render(&mut self, ui: &mut egui::Ui, is_fragment_tab: bool, font_size: f32) {
        let code = if is_fragment_tab {
            &mut self.fragment_code
        } else {
            &mut self.vertex_code
        };

        let shader_type = if is_fragment_tab { "frag" } else { "vert" };
        let editor_id = format!("{}_{}", self.kind.as_str().to_lowercase(), shader_type);

        shader_editor::render_shader_editor(ui, code, &editor_id, font_size);
    }

    /// Get both vertex and fragment shader code
    pub fn get_shaders(&self) -> (&str, &str) {
        (&self.vertex_code, &self.fragment_code)
    }

    /// Set the fragment shader code
    pub fn set_fragment(&mut self, code: String) {
        self.fragment_code = code;
    }

    /// Set the vertex shader code
    pub fn set_vertex(&mut self, code: String) {
        self.vertex_code = code;
    }

    /// Get the buffer kind
    #[allow(dead_code)]
    pub fn _kind(&self) -> BufferKind {
        self.kind
    }
}
