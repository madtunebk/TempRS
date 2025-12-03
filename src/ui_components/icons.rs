/// Icon system using simple Unicode glyphs for consistency
/// These are guaranteed to render identically across all platforms
/// Currently unused but kept for future UI enhancements

use eframe::egui::{self, Color32, FontId, Pos2, Align2};

/// Render an icon at the specified position
#[allow(dead_code)]
pub fn render_icon(ui: &mut egui::Ui, icon: Icon, pos: Pos2, size: f32, color: Color32) {
    ui.painter().text(
        pos,
        Align2::CENTER_CENTER,
        icon.glyph(),
        FontId::proportional(size),
        color,
    );
}

/// Available icons using reliable Unicode glyphs
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    // Navigation
    Home,
    History,
    Suggestions,
    Likes,
    Playlists,
    Search,
    NowPlaying,
    
    // Playback
    Play,
    Pause,
    Next,
    Previous,
    Shuffle,
    ShuffleOff,
    Repeat,
    RepeatOne,
    RepeatOff,
    
    // Actions
    Like,
    LikeFilled,
    Share,
    More,
    Add,
    Remove,
    Close,
    Check,
    
    // Status
    Loading,
    Error,
    Success,
    Info,
    Warning,
    
    // Media
    Volume,
    VolumeMuted,
    Music,
    Playlist,
    Album,
    Artist,
}

#[allow(dead_code)]
impl Icon {
    /// Get the Unicode glyph for this icon
    pub fn glyph(&self) -> &'static str {
        match self {
            // Navigation - using simple geometric shapes
            Icon::Home => "⌂",           // House
            Icon::History => "⏱",        // Clock
            Icon::Suggestions => "✦",    // Star
            Icon::Likes => "♥",          // Heart
            Icon::Playlists => "☰",      // Menu/List
            Icon::Search => "⌕",         // Magnifying glass
            Icon::NowPlaying => "♫",     // Music note
            
            // Playback
            Icon::Play => "▶",           // Triangle right
            Icon::Pause => "⏸",          // Pause bars
            Icon::Next => "⏭",           // Next track
            Icon::Previous => "⏮",       // Previous track
            Icon::Shuffle => "⤨",        // Shuffle arrows
            Icon::ShuffleOff => "→",     // Simple arrow
            Icon::Repeat => "⟲",         // Circular arrow
            Icon::RepeatOne => "⟳",      // Single repeat
            Icon::RepeatOff => "—",      // Dash
            
            // Actions
            Icon::Like => "♡",           // Empty heart
            Icon::LikeFilled => "♥",     // Filled heart
            Icon::Share => "⤴",          // Share arrow
            Icon::More => "⋯",           // Three dots
            Icon::Add => "+",            // Plus
            Icon::Remove => "−",         // Minus
            Icon::Close => "X",          // X
            Icon::Check => "✓",          // Checkmark
            
            // Status
            Icon::Loading => "⟳",        // Spinning arrow
            Icon::Error => "⚠",          // Warning triangle
            Icon::Success => "✓",        // Check
            Icon::Info => "ℹ",           // Info
            Icon::Warning => "⚠",        // Warning
            
            // Media
            Icon::Volume => "🔊",        // Speaker
            Icon::VolumeMuted => "🔇",   // Muted speaker
            Icon::Music => "♪",          // Single note
            Icon::Playlist => "☰",       // List
            Icon::Album => "◎",          // Disc
            Icon::Artist => "♫",         // Double note
        }
    }
    
    /// Get a descriptive name for accessibility
    pub fn name(&self) -> &'static str {
        match self {
            Icon::Home => "Home",
            Icon::History => "History",
            Icon::Suggestions => "Suggestions",
            Icon::Likes => "Likes",
            Icon::Playlists => "Playlists",
            Icon::Search => "Search",
            Icon::NowPlaying => "Now Playing",
            
            Icon::Play => "Play",
            Icon::Pause => "Pause",
            Icon::Next => "Next",
            Icon::Previous => "Previous",
            Icon::Shuffle => "Shuffle",
            Icon::ShuffleOff => "Shuffle Off",
            Icon::Repeat => "Repeat All",
            Icon::RepeatOne => "Repeat One",
            Icon::RepeatOff => "Repeat Off",
            
            Icon::Like => "Like",
            Icon::LikeFilled => "Liked",
            Icon::Share => "Share",
            Icon::More => "More",
            Icon::Add => "Add",
            Icon::Remove => "Remove",
            Icon::Close => "Close",
            Icon::Check => "Check",
            
            Icon::Loading => "Loading",
            Icon::Error => "Error",
            Icon::Success => "Success",
            Icon::Info => "Info",
            Icon::Warning => "Warning",
            
            Icon::Volume => "Volume",
            Icon::VolumeMuted => "Muted",
            Icon::Music => "Music",
            Icon::Playlist => "Playlist",
            Icon::Album => "Album",
            Icon::Artist => "Artist",
        }
    }
}

/// Helper to create an icon button
#[allow(dead_code)]
pub fn icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    size: f32,
    color: Color32,
    bg_color: Color32,
) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(icon.glyph())
                .size(size)
                .color(color)
        )
        .fill(bg_color)
    )
}
