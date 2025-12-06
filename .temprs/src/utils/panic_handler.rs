//! Panic Handling and Recovery
#![allow(dead_code)]
//!
//! Provides utilities to catch panics and convert them to recoverable errors,
//! preventing the application from crashing.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Result type for operations that might panic
pub type PanicResult<T> = Result<T, String>;

/// Catch panics from a closure and convert to Result
///
/// This allows operations that might panic (like WGPU validation errors)
/// to be handled gracefully without crashing the entire application.
///
/// # Example
/// ```ignore
/// let result = catch_panic(|| {
///     potentially_panicking_operation();
/// });
///
/// match result {
///     Ok(value) => // Success
///     Err(panic_msg) => // Handle panic gracefully
/// }
/// ```
pub fn catch_panic<F, T>(f: F) -> PanicResult<T>
where
    F: FnOnce() -> T + std::panic::UnwindSafe,
{
    match catch_unwind(f) {
        Ok(result) => Ok(result),
        Err(panic_info) => {
            // Extract panic message
            let message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred".to_string()
            };

            log::error!("Caught panic: {}", message);
            Err(message)
        }
    }
}

/// Catch panics from a mutable closure
///
/// Similar to `catch_panic` but allows mutable references.
/// Uses `AssertUnwindSafe` to bypass Rust's panic safety checks.
///
/// **WARNING**: Only use this when you're certain the mutable state
/// won't be left in an inconsistent state if a panic occurs.
pub fn catch_panic_mut<F, T>(f: F) -> PanicResult<T>
where
    F: FnOnce() -> T,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => Ok(result),
        Err(panic_info) => {
            let message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred".to_string()
            };

            log::error!("Caught panic (mut): {}", message);
            Err(message)
        }
    }
}

/// Format a panic message for user display
///
/// Extracts the most relevant information from a panic message
/// and formats it in a user-friendly way.
pub fn format_panic_message(panic_msg: &str) -> String {
    // WGPU errors often have "wgpu error:" prefix
    if panic_msg.contains("wgpu error:") {
        if let Some(start) = panic_msg.find("wgpu error:") {
            let error_part = &panic_msg[start..];
            return format!("GPU Error\n\n{}", error_part);
        }
    }

    // Shader validation errors
    if panic_msg.contains("Validation Error") {
        return format!("Shader Validation Error\n\n{}", panic_msg);
    }

    // Generic panic
    format!("Internal Error\n\n{}", panic_msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catch_panic_success() {
        let result = catch_panic(|| 42);
        assert_eq!(result, Ok(42));
    }

    #[test]
    fn test_catch_panic_failure() {
        let result = catch_panic(|| {
            panic!("Test panic");
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Test panic"));
    }

    #[test]
    fn test_catch_panic_mut() {
        let mut x = 10;
        let result = catch_panic_mut(|| {
            x += 5;
            x
        });
        assert_eq!(result, Ok(15));
    }

    #[test]
    fn test_format_wgpu_error() {
        let msg = "thread panicked with wgpu error: Validation Error in pipeline";
        let formatted = format_panic_message(msg);
        assert!(formatted.contains("GPU Error"));
        assert!(formatted.contains("Validation Error"));
    }
}
