// Screen modules - Full-screen views/windows
pub mod splash;
pub mod home;
pub mod now_playing;
pub mod search;
pub mod history;
pub mod suggestions;
pub mod likes;
pub mod user_playlists;

// Re-export for convenience
pub use splash::render_splash_screen;
pub use home::render_home_view;
pub use now_playing::render_now_playing_view;
pub use search::render_search_view;
pub use history::render_history_view;
pub use suggestions::render_suggestions_view;
pub use likes::render_likes_view;
pub use user_playlists::render_user_playlists_view;
