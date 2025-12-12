use std::any::Any;
use std::fmt;

/// Error type for shader compilation and pipeline errors
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum ShaderError {
    CompilationError(String),
    ValidationError(String),
    DeviceError(String),
    UnknownError(String),
}

impl fmt::Display for ShaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderError::CompilationError(msg) => write!(f, "Shader compilation error: {}", msg),
            ShaderError::ValidationError(msg) => write!(f, "Shader validation error: {}", msg),
            ShaderError::DeviceError(msg) => write!(f, "Device error: {}", msg),
            ShaderError::UnknownError(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for ShaderError {}

impl From<String> for ShaderError {
    fn from(s: String) -> Self {
        ShaderError::UnknownError(s)
    }
}

impl From<&str> for ShaderError {
    fn from(s: &str) -> Self {
        ShaderError::UnknownError(s.to_string())
    }
}

/// Convert a panic payload to a readable string
#[allow(dead_code)]
pub fn panic_to_string(e: Box<dyn Any + Send>) -> String {
    let any = &*e;
    if let Some(s) = any.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = any.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic occurred during shader compilation".to_string()
    }
}

/// Parse WGSL error message to extract useful information
#[allow(dead_code)]
pub fn parse_wgsl_error(error_msg: &str) -> ShaderError {
    // Check for common WGSL error patterns
    if error_msg.contains("expected") || error_msg.contains("unexpected") {
        ShaderError::CompilationError(error_msg.to_string())
    } else if error_msg.contains("validation") {
        ShaderError::ValidationError(error_msg.to_string())
    } else if error_msg.contains("device") {
        ShaderError::DeviceError(error_msg.to_string())
    } else {
        ShaderError::UnknownError(error_msg.to_string())
    }
}

/// Format shader error for display with line numbers if available
#[allow(dead_code)]
pub fn format_shader_error(error: &ShaderError) -> String {
    let msg = match error {
        ShaderError::CompilationError(msg) => msg,
        ShaderError::ValidationError(msg) => msg,
        ShaderError::DeviceError(msg) => msg,
        ShaderError::UnknownError(msg) => msg,
    };

    // Clean up the error message by removing excessive whitespace and formatting nicely
    let cleaned = msg
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    // Add type prefix
    let prefix = match error {
        ShaderError::CompilationError(_) => "Compilation Error",
        ShaderError::ValidationError(_) => "Validation Error",
        ShaderError::DeviceError(_) => "Device Error",
        ShaderError::UnknownError(_) => "Shader Error",
    };

    format!("{}\n\n{}", prefix, cleaned)
}

/// Extract line information from error message (e.g., "line 42")
#[allow(dead_code)]
fn extract_line_info(msg: &str) -> Option<String> {
    // Look for patterns like "line 42" or "42:5"
    for word in msg.split_whitespace() {
        if word.starts_with("line") {
            return Some(word.to_string());
        }
        if word.contains(':') && word.chars().next()?.is_ascii_digit() {
            return Some(format!("line {}", word));
        }
    }
    None
}
