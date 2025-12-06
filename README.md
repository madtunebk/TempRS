# TempRS - SoundCloud Desktop Player

A Rust-based desktop music player for SoundCloud with progressive audio streaming.

![TempRS Now Playing](images/08_now_playing_playlist.png)

> 📸 **[View all screenshots and detailed feature documentation →](images/README.md)**

## ⚠️ Setup Required

**Before building**: You need SoundCloud API credentials.

1. Create a `.env` file from the template:
```bash
cp .env.example .env
```

2. Add your SoundCloud API credentials to `.env`:
```env
SOUNDCLOUD_CLIENT_ID=your_client_id_here
SOUNDCLOUD_CLIENT_SECRET=your_client_secret_here
```

Credentials are loaded at compile time via `build.rs` and never committed to git.

## Build & Run

```bash
# Make sure .env file is configured first
cargo build --release
./target/release/TempRS
```

## Features

✅ **Progressive Audio Streaming**
- Streams audio directly from SoundCloud CDN without downloading full files
- Uses minimp3 for real-time MP3 decoding
- Low memory footprint (~2MB buffer with 5MB limit)
- Instant playback start
- Buffering state tracking & timeout detection (5s)

✅ **Real-time Audio Visualization**
- FFT-based frequency analysis (bass, mid, high bands)
- Dual-channel architecture: download + playback streams
- Continuous processing throughout entire track (not just buffering phase)
- Synchronized with seeking - no interruptions or desync
- Non-blocking: FFT runs in dedicated thread, never blocks audio playback
- Accurate beat detection locked to actual playback samples

✅ **Multi-Pass Shader System**
- Offscreen buffer rendering (Buffer A-D) with MainImage compositor
- Hot-reload workflow: edit external JSON shader → auto-reload in player (2s check)
- JSON format with base64 encoding support for safe storage
- WGSL validation with naga (early error detection with helpful messages)
- Graceful degradation: missing buffers render black, no crashes
- Auto-injection: uniforms, vertex shader, texture bindings (no boilerplate needed)
- Compatible with shader editor exports (see `docs/SETUP.md`)
- Demo shader included (`src/assets/shards/demo_multipass.json`)
- Single-pass fallback for backward compatibility

✅ **Smart Caching**
- Hybrid filesystem + SQLite metadata caching
- Artwork caching with placeholder tracking (prevents retry loops)
- Auto-cleanup (30 days + 100GB limit)
- No audio file storage (pure streaming)

✅ **Playback Controls**
- Play/Pause/Stop
- Next/Previous track
- Shuffle & Repeat modes
- Seeking (restarts stream at offset)
- Volume control with vertical popup slider
- Mute/unmute (right-click speaker icon)

✅ **Library Management**
- Like/unlike tracks (synced with SoundCloud API)
- Like/unlike playlists (synced with SoundCloud API)
- Playback history tracking (local SQLite database)
- Recently played tracks (no API calls needed)

✅ **Authentication**
- OAuth 2.0 with PKCE
- Machine-bound encrypted token storage (AES-256-GCM)
- Auto token refresh
- Machine fingerprinting (CPU + machine ID)

## Technical Stack

- **UI**: egui 0.33 / eframe (with wgpu backend)
- **Audio**: rodio 0.19 + minimp3 0.5
- **FFT Analysis**: rustfft 6.2 (real-time frequency analysis)
- **Shader System**: WGSL shaders via egui-wgpu with naga validation
- **HTTP**: reqwest 0.12 (with streaming support)
- **Async**: tokio 1.43
- **Storage**: rusqlite 0.32 (encrypted tokens, cache metadata, playback history)
- **Encryption**: AES-256-GCM (ring crate)

## Performance

**Resource Usage** (tested on AMD Ryzen 9 9900X / RTX 3060):
- **CPU**: <1% per thread during playback
- **RAM**: ~445MB total (comparable to Spotify/Discord)
- **Threads**: 20 (audio, FFT, download, HTTP clients, egui, WGPU, tokio workers)
- **Architecture**: Multi-threaded with zero blocking - audio, FFT, and UI all run independently

![Performance Metrics](images/09-htop.png)
*Real-world performance: <1% CPU usage per thread, ~445MB RAM, load average 0.42*

**Optimizations**:
- Dual-channel FFT: separate download + playback streams prevent blocking
- Progressive streaming: no full file buffering, minimal memory usage
- Efficient caching: hybrid filesystem + SQLite metadata
- Non-blocking UI: all heavy operations run in background threads

## Architecture

### Threading Model
- **Main thread**: Synchronous egui UI update loop
- **Background tasks**: `std::thread::spawn` + `tokio::Runtime` for async operations
- **Communication**: `mpsc::channel` for async results, `Arc<Mutex<T>>` for shared state
- **Memory**: Explicit resource cleanup with `drop()` when needed

### Progressive Audio Streaming
1. Request stream URL from SoundCloud API (OAuth authenticated)
2. Follow redirect to CDN (cf-media.sndcdn.com)
3. Stream audio chunks via HTTP
4. Decode MP3 frames progressively with minimp3
5. Feed decoded samples to rodio for playback

**Dual-channel FFT architecture:**
- Download thread → decode → audio_tx (playback) + fft_download_tx (visualization during buffering)
- Playback iterator → fft_playback_tx (visualization during actual playback)
- FFT thread processes both channels → bass/mid/high frequency analysis
- Result: Smooth playback + accurate visualization with zero stuttering

**Buffer management:**
- Buffering threshold: 44100 samples (~1 second)
- 5-second timeout detection for stuck streams
- Buffer limits: 5MB max, auto-trim to 2MB

**Seeking:**
- Calculate byte offset (assumes 128kbps MP3 ≈ 16KB/s)
- Request fresh redirect URL with Range header
- Start new stream from offset position

### Caching Strategy
- **Filesystem**: `~/.cache/TempRS/` - artwork/thumbnails (SHA256-named files)
- **SQLite**: `~/.cache/TempRS/cache.db` - metadata tracking (URLs, hashes, timestamps)
- **Placeholder tracking**: Prevents retry loops for 404s (`is_placeholder=1` flag)
- **Auto-cleanup**: 30 days old + 100GB limit at startup
- **No audio caching**: Pure streaming, no disk storage

### Storage Locations
- **Tokens**: `~/.config/TempRS/tokens.db` (AES-256-GCM encrypted)
- **History**: `~/.config/TempRS/playback_history.db` (local playback tracking)
- **Cache**: `~/.cache/TempRS/` (artwork + metadata)

## Recent Updates

### v0.2.1 (2025-12-05)
- **Environment variables**: .env file for credentials (build-time loading)
- **Social buttons**: Simplified to like-only on artwork, share in player bar
- **Widget ID fixes**: No more red error messages on duplicate tracks
- **Splash screen**: 2-second minimum display for smooth startup
- **Documentation**: Organized into docs/ folder, removed 10 outdated files

### v0.2.0 (2025-12-02)
- **Volume control**: Vertical popup slider (click to toggle, right-click to mute)
- **Playlist management**: Like/unlike playlists with API sync
- **Graceful shutdown**: Proper resource cleanup on exit
- **Audio improvements**: Buffering state tracking, timeout detection, buffer management
- **UI polish**: Removed duplicate badges, fixed character encoding, text-only toasts

## Screenshots

See [images/README.md](images/README.md) for detailed feature screenshots.

## Build & Run

```bash
# Make sure .env file is configured first
cargo build --release
./target/release/TempRS
```

## Project Structure

```
src/
├── app/
│   ├── player_app.rs      # Main app state & orchestration
│   ├── queue.rs            # Playback queue management
│   └── playlists.rs        # Playlist data models
├── ui_components/
│   ├── header.rs           # Top navigation bar
│   ├── layout.rs           # Shared layout wrapper (header/footer/sidebar)
│   ├── player.rs           # Playback controls footer
│   ├── playlist_sidebar.rs # Queue sidebar with track list
│   ├── helpers.rs          # UI utility functions (social buttons, track cards)
│   ├── search_bar.rs       # Search input component
│   ├── toast.rs            # Toast notification system
│   ├── colors.rs           # Color constants
│   └── icons.rs            # Icon rendering utilities
├── screens/
│   ├── splash.rs           # Splash screen with WGSL shader (2s minimum)
│   ├── likes.rs            # Liked tracks with unlike buttons
│   ├── user_playlists.rs   # Playlists tab with unlike buttons
│   ├── history.rs          # Playback history view
│   ├── now_playing.rs      # Now playing full screen view
│   ├── suggestions.rs      # Suggestions/Related tracks view
│   ├── home/               # Home screen modules
│   │   ├── mod.rs
│   │   ├── recently_played.rs
│   │   ├── recommendations.rs
│   │   └── suggestions.rs
│   └── search/             # Search screen modules
│       ├── mod.rs
│       ├── filters.rs
│       └── results.rs
├── api/
│   ├── likes.rs            # Like/unlike tracks & playlists
│   ├── playlists.rs        # Playlist fetching
│   ├── tracks.rs           # Track streaming & metadata
│   ├── search.rs           # Search endpoints
│   ├── users.rs            # User profile endpoints
│   └── activities.rs       # Activity stream endpoints
├── utils/
│   ├── audio_controller.rs # Audio thread management
│   ├── mediaplay.rs        # Streaming & MP3 decoding (progressive streaming)
│   ├── oauth.rs            # OAuth 2.0 + PKCE flow
│   ├── token_store.rs      # AES-256-GCM encrypted storage
│   ├── token_helper.rs     # Token validation & refresh
│   ├── fingerprint.rs      # Machine fingerprinting
│   ├── cache.rs            # Hybrid caching (filesystem + DB)
│   ├── playback_history.rs # Local playback tracking
│   ├── pipeline.rs         # Single-pass shader rendering
│   ├── multi_buffer_pipeline.rs # Multi-pass shader rendering (Buffer A-D)
│   ├── shader_json.rs      # JSON shader parser with base64 support
│   ├── shader_validator.rs # WGSL validation with naga
│   ├── shader_constants.rs # Centralized shader boilerplate
│   ├── errors.rs           # Shader error types
│   ├── audio_analyzer.rs   # Audio analysis utilities
│   ├── audio_fft.rs        # FFT audio visualization
│   ├── artwork.rs          # Artwork loading & caching
│   ├── clipboard.rs        # Clipboard operations
│   ├── formatting.rs       # Time/number formatting
│   ├── http.rs             # HTTP client utilities
│   └── track_filter.rs     # Track filtering (streamable checks)
├── models/
│   ├── track.rs            # Track data structures
│   ├── playlist.rs         # Playlist data structures
│   ├── user.rs             # User data structures
│   ├── activity.rs         # Activity data structures
│   └── responses.rs        # API response wrappers
├── data/
│   └── home_data.rs        # Home screen data management
├── shaders/
│   ├── splash_bg.wgsl      # Splash screen background shader
│   ├── track_metadata_bg.wgsl # Track metadata background shader
│   ├── plasma.wgsl         # Plasma effect shader
│   ├── multipass_*.wgsl    # Placeholder multi-pass shaders
│   └── shader.wgsls        # Legacy multi-pass shader export
├── assets/
│   ├── shards/
│   │   ├── demo_multipass.json # Demo multi-pass shader (JSON format)
│   │   └── shader_format.md    # JSON shader format documentation
│   └── fonts/              # Icon fonts and regular fonts
└── app_state.rs            # Global app state (Arc<RwLock>)
```

## Cache Locations

- **Tokens**: `~/.config/TempRS/tokens.db` (AES-256-GCM encrypted)
- **Cache DB**: `~/.cache/TempRS/cache.db` (metadata: URLs, hashes, timestamps)
- **Artwork**: `~/.cache/TempRS/artwork/` (SHA256-named files)
- **Sidebar Artwork**: `~/.cache/TempRS/sidebar_artwork/`
- **Shaders**: `~/.cache/TempRS/shaders/shader.json` (hot-reloadable shader exports)
- **Playback History**: `~/.config/TempRS/playback_history.db` (local tracking)
- **No audio files stored** (streaming only)

## Shader System

TempRS supports audio-reactive WGSL shaders with multi-pass rendering:

- **Single-pass**: Simple shaders with one fragment function (backward compatible)
- **Multi-pass**: 4 offscreen buffers (Buffer A-D) + MainImage compositor
- **Hot-reload**: Edit shader JSON in `~/.cache/TempRS/shaders/` → auto-reload every 2s
- **Editor integration**: Compatible with shader editor exports (see `docs/SETUP.md`)

For shader pipeline specification and editor setup, see:
- [`docs/PIPELINE_SPEC.md`](docs/PIPELINE_SPEC.md) - Technical specification
- [`docs/SETUP.md`](docs/SETUP.md) - Editor integration guide
- [`src/assets/shards/shader_format.md`](src/assets/shards/shader_format.md) - JSON format

## Contributing

See [docs/TODO.md](docs/TODO.md) for active tasks and future improvements.

## License

This project is for personal/educational use. SoundCloud API usage requires valid credentials.
- minimp3: ~50KB addition to binary
- All dependencies statically linked

## License

See project license file.
