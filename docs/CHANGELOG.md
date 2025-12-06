# TempRS - Changelog

All notable changes to this project will be documented in this file.

## [0.2.2] - 2025-12-06

### Added
- **Multi-Pass Shader Pipeline**: Advanced shader rendering system
  - Support for 4 offscreen buffers (Buffer A-D) + MainImage compositor
  - JSON shader format with base64 encoding support
  - Shader hot-reload system with checksum-based file watching (2s interval)
  - Naga-based WGSL validation with detailed error messages
  - Graceful degradation when buffer shaders are missing/invalid
  - Auto-injection of boilerplate (Uniforms, VSOut, vertex shader, texture bindings)
  - 5 shader entry points: vs_main, fs_main, fs_buffer_a-d
  - Shared sampler with linear filtering and clamp-to-edge addressing
  - Demo multi-pass shader included (`demo_multipass.json`)

- **Shader Utilities**: Comprehensive shader infrastructure
  - `errors.rs`: Typed shader error system (Compilation/Validation/Device/Unknown)
  - `pipeline.rs`: Single-pass shader rendering (backward compatible)
  - `multi_buffer_pipeline.rs`: Multi-pass rendering with offscreen textures
  - `shader_json.rs`: JSON shader parser with base64 decode support
  - `shader_validator.rs`: Naga-based WGSL validation with helpful messages
  - `shader_constants.rs`: Centralized boilerplate definitions

- **Shader Integration**: Editor export compatibility
  - Load shaders from `~/.cache/TempRS/shaders/shader.json`
  - Fallback to embedded demo shader if cache missing
  - Hot-reload workflow: edit in external editor → save → auto-reload in player
  - Support for both single-pass and multi-pass exports
  - Seamless upgrade path from old single-pass shaders

- **Documentation**: Shader pipeline and setup guides
  - `PIPELINE_SPEC.md`: Technical specification for multi-pass rendering
  - `SETUP.md`: Integration guide for shader editor + TempRS workflow
  - `README.md`: Shader file format and usage documentation

### Changed
- **Now Playing View**: Multi-pass shader background support
  - Prefers multi-pass shader if available, falls back to single-pass
  - Both shaders share same audio energy data (bass/mid/high)
  - Semi-transparent overlay for text readability

- **Splash Screen**: Updated audio uniforms
  - Changed from hardcoded (0.0) to shared app audio energy
  - Consistent shader interface across all screens

- **Shader System Architecture**: Modular and maintainable
  - Split old `shader.rs` into 6 specialized modules
  - Clean separation of concerns (pipeline/validation/errors/JSON)
  - Re-exports in `utils/mod.rs` for easy importing
  - Better error messages with naga integration

### Fixed
- **Shader Validation**: Early error detection
  - Catch missing uniforms struct before pipeline creation
  - Validate entry points and attributes
  - Detailed error messages with line numbers (when available)
  - Prevent crashes from malformed shaders

- **Colors Module**: Missing `#[allow(dead_code)]` on OVERLAY_BADGE
  - Fixed compiler warning for unused constant

## [0.2.1] - 2025-12-05

### Added
- **Environment Variables**: Credential management via .env file
  - Added dotenvy dependency for .env support
  - build.rs loads credentials at compile time
  - .env.example template for setup
  - Credentials no longer hardcoded in source

- **Splash Screen Timer**: Smooth startup experience
  - 2-second minimum splash screen display
  - Prevents window glitches during initialization
  - Active timer checking with repaint requests
  - Debug logging for elapsed time tracking

### Changed
- **Social Buttons**: Simplified artwork interaction
  - Removed share button from track artwork (only like button now)
  - Share functionality exclusive to bottom player bar
  - Top-left corner placement for like button
  - Fixed widget ID clashes using artwork position coordinates
  - Unique IDs prevent red error messages when same track appears multiple times

- **Screen Headers**: Consistent styling across all views
  - Uniform headers on History, Suggestions, Likes, Playlists
  - 24px font size, white color, bold weight
  - Removed duplicate headers from views

- **Documentation Organization**: Clean repo structure
  - Moved all .md files to docs/ folder (except README.md)
  - Removed 10 outdated/completed feature docs
  - Kept 9 active documentation files
  - Removed obsolete credentials.example.rs

### Fixed
- **Widget ID Conflicts**: Red error messages on home screen
  - Social buttons now use artwork rect position for unique IDs
  - Fixed "First use of widget ID" errors
  - Prevents clashes when same track in multiple sections

### Infrastructure
- **.gitignore Updates**: Cleaner repository
  - Added commit.sh, test_play_history.sh, tools.txt to ignore
  - Local scripts no longer tracked in git
  - Removed from both remotes (origin + github)

## [0.2.0] - 2025-12-02

### Added
- **Volume Popup Slider**: Vertical popup volume control (140px tall)
  - Click speaker icon to toggle popup
  - Right-click speaker to mute/unmute
  - Shadow layers and orange accent styling
  - Auto-unmute when adjusting volume while muted
  
- **Playlist Like/Unlike**: Full playlist management
  - Heart button on playlist cards in Playlists tab
  - API integration: `like_playlist()` / `unlike_playlist()`
  - Optimistic UI updates with background API sync
  - Toast notifications for actions
  - `liked_playlist_ids` HashSet tracking

- **Graceful Shutdown**: Proper resource cleanup on exit
  - Stops audio playback
  - Saves playback settings
  - Clears receivers, textures, and caches
  - Releases OAuth and shader resources
  - Logs each cleanup step

### Changed
- **Audio Streaming**: Enhanced reliability and performance
  - Added buffering state tracking (44100 sample threshold)
  - 5-second timeout detection for stuck streams
  - Buffer limits: 5MB max, trims to 2MB when exceeded
  - Frame-index tracking to avoid duplicate sends
  
- **Unlike Icons**: Improved visual feedback
  - Changed unliked icon from 🤍 (empty heart) to 💔 (broken heart)
  - Consistent across Likes screen and playlist cards
  
- **Toast Notifications**: Cleaner messaging
  - Removed all emojis from notifications
  - Text-only messages: "Added to Liked tracks", "Removed from Liked tracks"
  
- **Character Encoding**: Fixed display issues
  - Replaced × character with x/X throughout UI
  - Prevents encoding problems on different systems

### Removed
- **Duplicate UI Elements**: Cleaner track cards
  - Removed duplicate heart badges from Likes screen
  - Heart button now only appears in top-left corner
  - Badge logic: 💜 for liked tracks, 🎤 for uploaded tracks (no overlap)
  
- **Queue Heart Buttons**: Simplified queue sidebar
  - Removed heart buttons from queue track cards
  - Player footer heart button is sufficient

### Fixed
- **Audio Sync Issues**: Rare endless stuck/choppy audio
  - Buffering state prevents premature "finished" detection
  - Timeout detection catches hung streams (5-second threshold)
  - Buffer trimming prevents memory bloat
  
- **Incremental Decoding**: Reverted breaking change
  - Incremental MP3 decoding caused choppy audio
  - Returned to frame-index method (original working implementation)

### Infrastructure
- **Git Repository**: Updated remote configuration
  - Remote: `ssh://gitea@gitea.home.cornfield/nobus/TempRS.git`
  - Clean SSH format (no custom ports)
  - All changes committed and pushed

## [0.1.0] - 2025-12-01

### Added
- **Social Buttons**: Like/unlike and share functionality
  - Heart button in player footer, Likes screen, and Now Playing views
  - Share button copies track URL to clipboard
  - API integration: `like_track()` / `unlike_track()`
  - Optimistic UI updates
  - `liked_track_ids` HashSet tracking
  - Fetch liked track IDs on startup

- **User Playlists Screen**: Dedicated playlists view
  - Renamed from `playlists.rs` to `user_playlists.rs` for clarity
  - Grid layout with playlist cards (artwork, title, track count)
  - Click to load playlist into queue and play
  - Fetch both created and liked playlists from SoundCloud
  - Centered layout matching Suggestions screen

- **Playback History Screen**: Local tracking and display
  - SQLite database: `~/.config/TempRS/playback_history.db`
  - Tracks: play count, last played timestamp, track metadata
  - Grid view with pagination
  - Sort options: Recent first, Most played, Alphabetical
  - Search filter by title/artist
  - No API calls needed (local data)

- **Hybrid Caching System**: Filesystem + SQLite metadata
  - Cache database: `~/.cache/TempRS/cache.db`
  - Tracks: URL, cache type, file hash, timestamp, placeholder flag
  - Prevents retry loops for 404s
  - Auto-cleanup: 30 days + 100GB limit
  - Runs once at startup in background thread

### Changed
- **Likes Screen Layout**: Improved centering and consistency
  - Uses `calculate_grid_layout()` for dynamic sizing
  - Matches Suggestions screen pattern
  - Improved title: 24.0 size with strong weight
  - Centered empty states with icons

### Fixed
- **Like State on Startup**: Immediate availability
  - Fetches `liked_track_ids` after authentication
  - Populates before user visits Likes tab
  - Heart icons show correct state immediately

## [0.0.1] - 2025-11-28

### Added
- **Progressive Audio Streaming**: True HTTP streaming
  - No full file downloads
  - minimp3 for real-time MP3 decoding
  - Low memory footprint (~2MB buffer)
  - Instant playback start
  
- **OAuth Authentication**: Secure token management
  - OAuth 2.0 with PKCE flow
  - Machine-bound encrypted token storage (AES-256-GCM)
  - Machine fingerprinting (CPU + machine ID)
  - Auto token refresh
  - SQLite: `~/.config/TempRS/tokens.db`

- **Basic Playback Controls**:
  - Play/Pause/Stop
  - Next/Previous track
  - Shuffle & Repeat modes
  - Seeking (restarts stream at offset)
  - Volume control (horizontal slider)

- **Home Screen**: Now Playing view
  - Large artwork display
  - Track metadata (title, artist, genre)
  - Social buttons placeholder
  - Recently Played section
  - Recommendations section

- **Search Functionality**: Tracks and playlists
  - Real-time search with SoundCloud API
  - Grid layout for results
  - Pagination support
  - Click to play or load playlist

- **Queue Management**: Playback queue sidebar
  - Displays current queue with artwork thumbnails
  - Shuffle preservation (maintains original track order)
  - Click track to jump to position
  - Auto-scroll to current track
  - Orange border for playing track

- **Splash Screen**: WGSL shader background
  - Custom shader pipeline with egui-wgpu
  - Animated background effect
  - Smooth transition to main screen

### Technical
- **egui/eframe**: Immediate-mode GUI (v0.33)
- **rodio**: Audio playback (v0.19)
- **minimp3**: MP3 decoding (v0.5)
- **tokio**: Async runtime (v1.43)
- **rusqlite**: SQLite storage (v0.32)
- **reqwest**: HTTP client with streaming (v0.12)
- **ring**: AES-256-GCM encryption

### Performance
- Memory: ~2MB audio buffer (5MB limit with trim)
- Bandwidth: Streams only what you listen to
- Startup: Instant playback (no download wait)
- Disk: No audio files cached (artwork only)

---

## Version Format

Versions follow Semantic Versioning (SemVer):
- **MAJOR**: Incompatible API/architecture changes
- **MINOR**: New features, backward-compatible
- **PATCH**: Bug fixes, backward-compatible

## Categories

- **Added**: New features
- **Changed**: Changes to existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Vulnerability fixes
- **Infrastructure**: Build/deployment/repository changes
