/// Services module - business logic layer
///
/// Services contain reusable business logic that can be called from UI components.
/// They help reduce duplication and keep the UI layer thin.

pub mod social;

// Re-export commonly used types
pub use social::{LikeTarget, toggle_like};
