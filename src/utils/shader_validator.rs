//! WGSL Shader Validation
#![allow(dead_code)]
//! 
//! Validates shaders before passing to pipeline to catch errors early
//! and provide helpful error messages in the UI.
//! 
//! Uses WGSL definitions from wgsl_syntax module for consistency.

use crate::utils::ShaderError;

// WGSL Language Constants (aligned with wgsl_syntax.rs)
const REQUIRED_KEYWORDS: &[&str] = &["fn", "struct", "var"];
const REQUIRED_TYPES: &[&str] = &["f32", "vec2", "vec4"];

/// Validates a WGSL shader source code
/// 
/// Performs multiple validation checks:
/// 1. Uniforms struct matches expected structure
/// 2. Required entry points exist (vs_main, fs_main)
/// 3. WGSL syntax validation via naga
/// 4. Shader logic validation
pub fn validate_shader(wgsl_src: &str) -> Result<(), ShaderError> {
    validate_shader_with_entry_point(wgsl_src, "fs_main")
}

/// Validates a WGSL shader with a specific fragment entry point
/// 
/// Used for multi-buffer rendering where each buffer may have different entry points
/// (e.g., fs_main, fs_buffer_a, fs_buffer_b, etc.)
pub fn validate_shader_with_entry_point(wgsl_src: &str, entry_point: &str) -> Result<(), ShaderError> {
    // 1. Check shader is not empty
    if wgsl_src.trim().is_empty() {
        return Err(ShaderError::ValidationError(
            "Shader source is empty".to_string(),
        ));
    }

    // 2. Validate Uniforms struct
    validate_uniforms_struct(wgsl_src)?;

    // 3. Validate basic WGSL language constructs
    validate_wgsl_constructs(wgsl_src)?;

    // 4. Validate required attributes and entry points
    validate_entry_points_with_fragment(wgsl_src, entry_point)?;

    // 5. Validate WGSL syntax with naga
    validate_wgsl_syntax(wgsl_src)?;

    Ok(())
}

/// Validate basic WGSL language constructs are present
fn validate_wgsl_constructs(wgsl_src: &str) -> Result<(), ShaderError> {
    // Check for essential keywords
    for keyword in REQUIRED_KEYWORDS {
        if !wgsl_src.contains(keyword) {
            return Err(ShaderError::ValidationError(
                format!("Shader missing required WGSL keyword: '{}'", keyword)
            ));
        }
    }
    
    // Check for essential types
    for type_name in REQUIRED_TYPES {
        if !wgsl_src.contains(type_name) {
            return Err(ShaderError::ValidationError(
                format!("Shader missing required WGSL type: '{}'", type_name)
            ));
        }
    }
    
    Ok(())
}

/// Validate that the Uniforms struct matches our expected structure
fn validate_uniforms_struct(wgsl_src: &str) -> Result<(), ShaderError> {
    // Required fields (must be present)
    let required_fields = [
        "time: f32",
        "audio_bass: f32",
        "audio_mid: f32",
        "audio_high: f32",
        "resolution: vec2<f32>",
    ];
    
    // Check if shader defines a Uniforms struct
    if !wgsl_src.contains("struct Uniforms") {
        return Err(ShaderError::ValidationError(
            "Shader must define a 'struct Uniforms' matching the pipeline structure.\n\nExpected:\nstruct Uniforms {\n    time: f32,\n    audio_bass: f32,\n    audio_mid: f32,\n    audio_high: f32,\n    resolution: vec2<f32>,\n    gamma: f32,\n    _pad0: f32,\n}".to_string()
        ));
    }
    
    // Extract struct definition
    if let Some(start) = wgsl_src.find("struct Uniforms") {
        if let Some(struct_content) = wgsl_src[start..].find('{') {
            let start_brace = start + struct_content;
            if let Some(end_brace) = wgsl_src[start_brace..].find('}') {
                let struct_body = &wgsl_src[start_brace + 1..start_brace + end_brace];
                
                // Check required fields only (gamma is optional for backward compatibility)
                for field in &required_fields {
                    if !struct_body.contains(field) {
                        return Err(ShaderError::ValidationError(format!(
                            "Uniforms struct mismatch!\n\nMissing field: {}\n\nRequired fields:\n{:?}",
                            field, required_fields
                        )));
                    }
                }
                
                // Validate padding format (either old or new is fine)
                let has_new_format = struct_body.contains("gamma: f32") && struct_body.contains("_pad0: f32");
                let has_old_format = struct_body.contains("_pad0: vec2<f32>");
                
                if !has_new_format && !has_old_format {
                    log::warn!("Shader uses non-standard padding format - this may cause issues");
                }
            }
        }
    }
    
    // Check binding declaration
    if !wgsl_src.contains("@group(0) @binding(0)") || !wgsl_src.contains("var<uniform> uniforms: Uniforms") {
        return Err(ShaderError::ValidationError(
            "Missing uniform binding declaration.\n\nRequired:\n@group(0) @binding(0) var<uniform> uniforms: Uniforms;".to_string()
        ));
    }
    
    Ok(())
}

/// Validate required shader entry points and attributes
fn validate_entry_points(wgsl_src: &str) -> Result<(), ShaderError> {
    validate_entry_points_with_fragment(wgsl_src, "fs_main")
}

/// Validate required shader entry points with custom fragment entry point name
fn validate_entry_points_with_fragment(wgsl_src: &str, fragment_entry: &str) -> Result<(), ShaderError> {
    // Check for @vertex and @fragment attributes (from REQUIRED_ATTRIBUTES)
    if !wgsl_src.contains("@vertex") {
        return Err(ShaderError::ValidationError(
            "Shader missing @vertex attribute".to_string(),
        ));
    }

    if !wgsl_src.contains("@fragment") {
        return Err(ShaderError::ValidationError(
            "Shader missing @fragment attribute".to_string(),
        ));
    }

    // Validate vertex entry point exists (flexible name check)
    if !wgsl_src.contains("fn vs_main") {
        return Err(ShaderError::ValidationError(
            "Shader missing vertex entry point 'fn vs_main'.\n\nRequired:\n@vertex\nfn vs_main(@builtin(vertex_index) vertex_index: u32) -> YourVertexOutput".to_string(),
        ));
    }

    // Validate fragment entry point exists (flexible name check)
    let fragment_fn = format!("fn {}", fragment_entry);
    if !wgsl_src.contains(&fragment_fn) {
        return Err(ShaderError::ValidationError(
            format!("Shader missing fragment entry point '{}'.\n\nRequired:\n@fragment\nfn {}(@location(0) coords: vec2<f32>) -> @location(0) vec4<f32>", fragment_fn, fragment_entry)
        ));
    }

    // Check for required attributes in vertex output struct
    let required_attrs = ["@builtin(position)", "@location(0)"];
    for attr in required_attrs {
        if !wgsl_src.contains(attr) {
            return Err(ShaderError::ValidationError(
                format!("Shader missing required attribute: {}", attr)
            ));
        }
    }
    
    // Check for vertex output struct pattern with required types
    if !wgsl_src.contains("vec4<f32>") || !wgsl_src.contains("vec2<f32>") {
        return Err(ShaderError::ValidationError(
            "Shader missing vertex output struct with required types.\n\nExample:\nstruct VSOut {\n    @builtin(position) pos: vec4<f32>,\n    @location(0) uv: vec2<f32>,\n}".to_string(),
        ));
    }

    Ok(())
}

/// Validate WGSL syntax using naga parser
fn validate_wgsl_syntax(wgsl_src: &str) -> Result<(), ShaderError> {
    log::debug!("Validating WGSL with naga parser");
    
    // Parse WGSL
    let module = match naga::front::wgsl::parse_str(wgsl_src) {
        Ok(module) => {
            log::debug!("Naga parse successful");
            module
        }
        Err(parse_error) => {
            let error_msg = format!("WGSL Parse Error:\n{}", parse_error.emit_to_string(wgsl_src));
            log::error!("Shader parse failed: {}", error_msg);
            return Err(ShaderError::ValidationError(error_msg));
        }
    };

    // Validate the module
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );

    if let Err(validation_error) = validator.validate(&module) {
        let error_msg = format!("WGSL Validation Error:\n{}", validation_error.emit_to_string(wgsl_src));
        log::error!("Shader validation failed: {}", error_msg);
        return Err(ShaderError::ValidationError(error_msg));
    }

    log::debug!("Naga validation passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_shader() {
        let result = validate_shader("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_uniforms() {
        let shader = r#"
            @vertex
            fn vs_main() -> @builtin(position) vec4<f32> {
                return vec4<f32>(0.0, 0.0, 0.0, 1.0);
            }
            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return vec4<f32>(1.0, 0.0, 0.0, 1.0);
            }
        "#;
        let result = validate_shader(shader);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_entry_points() {
        let shader = r#"
            struct Uniforms {
                time: f32,
                audio_bass: f32,
                audio_mid: f32,
                audio_high: f32,
                resolution: vec2<f32>,
                _pad0: vec2<f32>,
            }
            @group(0) @binding(0) var<uniform> uniforms: Uniforms;
        "#;
        let result = validate_shader(shader);
        assert!(result.is_err());
    }
}
