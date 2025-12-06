pub mod audio;
pub mod audio_analyzer;
pub mod audio_file;
pub mod errors;
pub mod fonts;
pub mod image_loader;
pub mod monitors;
pub mod multi_buffer_pipeline;
pub mod notification;
pub mod panic_handler;
pub mod pipeline;
pub mod shader_constants;
pub mod shader_json;
pub mod shader_validator;
pub mod text;
pub mod theme;
pub mod toast;  // DEPRECATED: Use notification module instead
pub mod wgsl_syntax;

pub use errors::{format_shader_error, ShaderError};
pub use fonts::register_error_fonts;
pub use monitors::detect_primary_monitor_xrandr;
pub use multi_buffer_pipeline::{BufferKind, MultiPassCallback, MultiPassPipelines};
pub use notification::NotificationManager;
pub use panic_handler::{catch_panic_mut, format_panic_message};
pub use shader_constants::*;
pub use shader_json::ShaderJson;
pub use shader_validator::validate_shader;
pub use theme::apply_editor_theme;
// DEPRECATED: toast module kept for backward compatibility only
// Use NotificationManager from notification module instead
