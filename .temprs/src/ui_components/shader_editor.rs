//! Shared shader editor component used by all buffer tabs
//!
//! This module provides a reusable code editor widget with WGSL syntax highlighting,
//! eliminating the need for separate editor implementations per buffer.

use eframe::egui;

#[cfg(feature = "code_editor")]
use crate::utils::wgsl_syntax;

/// Renders a WGSL shader editor with consistent styling and features
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `code` - Mutable reference to the shader code string
/// * `editor_id` - Unique identifier for this editor instance
/// * `font_size` - Font size for the editor
pub fn render_shader_editor(
    ui: &mut egui::Ui,
    code: &mut String,
    editor_id: &str,
    font_size: f32,
) {
    ui.set_min_height(ui.available_height());

    #[cfg(feature = "code_editor")]
    {
        egui_code_editor::CodeEditor::default()
            .id_source(editor_id)
            .with_fontsize(font_size)
            .with_theme(egui_code_editor::ColorTheme::GITHUB_DARK)
            .with_syntax(wgsl_syntax::wgsl())
            .with_numlines(true)
            .vscroll(true)
            .auto_shrink(false)
            .show(ui, code);
    }

    #[cfg(not(feature = "code_editor"))]
    {
        ui.add(
            egui::TextEdit::multiline(code)
                .id(egui::Id::new(editor_id))
                .font(egui::TextStyle::Monospace)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(30),
        );
    }
}
