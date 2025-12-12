pub mod player_app;
pub mod playlists;
pub mod queue;
pub mod shader_manager;

pub use player_app::MusicPlayerApp;

// Re-export home data for backwards compatibility
pub use crate::data::home_data as home;
