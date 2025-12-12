// Data layer - aggregates data from database and API

pub mod home_data;

// Re-export main types (warnings suppressed - used by other modules)
#[allow(unused_imports)]
pub use home_data::{fetch_recently_played_async, fetch_recommendations_async, HomeContent};
