# TempRS - SoundCloud Desktop Player

Rust desktop music player for SoundCloud using egui/eframe (immediate-mode GUI), rodio for audio, and progressive MP3 streaming.

**Stack**: egui 0.33, rodio 0.19, tokio 1.43, rusqlite 0.32, minimp3 0.5, egui-wgpu 0.33  
**Platform**: Linux primary, Windows compatible  
**Build**: `cargo run --release --bin TempRS` (always use `--release` for performance)

## Architecture Overview

### Core Components

**MusicPlayerApp** (`src/app/player_app.rs`) - Central UI state orchestrator
- Owns all UI state, coordinates AudioController/OAuth/background tasks
- Uses `mpsc` channels: store `Option<Receiver<T>>`, poll with `try_recv()` in `update()` loop
- Screen routing: Splash → Main (Home/NowPlaying/Search/History/Likes/Playlists)

**AudioController** (`src/utils/audio_controller.rs`) - Dedicated audio thread
- Command pattern via `mpsc::Sender<AudioCommand>` (Play/Pause/Seek/SetVolume)
- Runs Tokio runtime in dedicated thread (NOT in main thread)
- Shares state: `Arc<Mutex<Duration>>` for position/duration

**AudioPlayer** (`src/utils/mediaplay.rs`) - Progressive MP3 streaming
- HTTP streaming → minimp3 decode → rodio playback via `StreamingSource` iterator
- Background thread fetches/decodes, sends i16 samples via channel
- Seeking: stops stream, calculates byte offset, fetches fresh URL with Range header
- 5MB buffer limit, 5s timeout detection, no disk caching (pure streaming)

**PlaybackQueue** (`src/app/queue.rs`) - Queue/shuffle/repeat logic
- `original_tracks`: immutable playlist, `current_queue`: shuffled indices
- Preserves current track position when toggling shuffle
- Filters non-playable tracks (`streamable != true`) on load

**AppState** (`src/app_state.rs`) - Shared state `Arc<RwLock<AppStateInner>>`
- Volume/shuffle/repeat settings, token expiry, NOT persisted to disk

### Critical Threading Pattern

egui's `update()` is synchronous. For async operations:

```rust
// 1. Create channel, store receiver in MusicPlayerApp field
let (tx, rx) = channel();
self.artwork_rx = Some(rx);

// 2. Spawn thread with Tokio runtime (NOT async in main thread)
std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let data = fetch_artwork().await;
        let _ = tx.send(data); // Send result back
    });
});

// 3. Poll in update() loop (non-blocking)
if let Some(rx) = &self.artwork_rx {
    if let Ok(image) = rx.try_recv() {
        self.artwork_texture = Some(ctx.load_texture(...)); // Process in main thread
        self.artwork_rx = None; // Clear receiver
    }
}
```

**Memory**: Explicitly `drop()` resources (e.g., `AudioPlayer`) to free memory immediately.

### Security & Storage

**Token Storage** (`src/utils/token_store.rs`)
- AES-256-GCM encryption using machine fingerprint-derived key
- SQLite at `~/.config/TempRS/tokens.db`, validates fingerprint on load
- Client ID/secret obfuscated as byte arrays in `player_app.rs`

**Machine Fingerprinting** (`src/utils/fingerprint.rs`)
- Linux: `/etc/machine-id` + CPU info → SHA256
- Windows: Registry `MachineGuid` + CPU → SHA256
- Used for PKCE `code_verifier` + token encryption

**Hybrid Caching** (`src/utils/cache.rs`)
- Filesystem: `~/.cache/TempRS/{artwork,sidebar_artwork}/` (SHA256-named files)
- SQLite metadata: `cache.db` with `(url, cache_type, file_hash, cached_at, is_placeholder)`
- `is_placeholder=1` flag prevents retry loops for 404s
- Auto-cleanup: 30 days + 100GB limit on startup (background thread)
- **No audio caching** - only artwork/thumbnails

**Playback History** (`src/utils/playback_history.rs`)
- Local SQLite at `~/.config/TempRS/playback_history.db`
- Powers Recently Played without API calls, supports pagination

**OAuth Flow** (`src/utils/oauth.rs`)
- PKCE with machine fingerprint → browser → `tiny_http` callback (localhost:3000) → token exchange

## Module Structure

- `app/`: Screen state management (`player_app.rs`, `queue.rs`, `playlists.rs`)
- `screens/`: Full views (`home/`, `search/`, `history.rs`, `now_playing.rs`, `splash.rs`)
- `ui_components/`: Reusable widgets (`header.rs`, `player.rs`, `layout.rs`)
- `utils/`: Infrastructure (OAuth, caching, audio, HTTP, fingerprinting)
- `models/`: Data structs with serde (Track, Playlist, User)
- `api/`: SoundCloud endpoints (`tracks.rs`, `playlists.rs`, `likes.rs`, `search.rs`)

## Key Patterns & Conventions

### UI Rendering
- Screen functions: `pub fn render_*_view(app: &mut MusicPlayerApp, ui: &mut egui::Ui, ctx: &egui::Context)`
- Layout wrapper: `ui_components/layout.rs::render_with_layout()` adds header/footer/sidebar
- Grid layouts: `egui::Grid::new(id).spacing([0.0, 0.0])` for structured multi-column UIs
- Centering: `Layout::centered_and_justified(LeftToRight)`
- Buttons: 40x40px, 20px rounding, `#2D2D32` inactive fill

### State Management
- Shared state: `Arc<Mutex<T>>` or `Arc<RwLock<T>>`
- UI state: owned by `MusicPlayerApp`, passed to render functions
- Track filtering: use `utils/track_filter.rs::is_track_playable()` to skip non-streamable

### Custom Shaders
- WGSL shaders via `egui-wgpu` (requires eframe `wgpu` feature)
- Initialize in `MusicPlayerApp` field, render via `CallbackTrait`
- Use `egui_wgpu::wgpu::*` types, NOT standalone `wgpu` crate
- Example: `src/shaders/splash_bg.wgsl`

### Logging
- `env_logger` initialized in `main()`, use `log::info!`, `log::error!`, etc.
- Set `RUST_LOG=debug` for verbose output

## Critical Gotchas

- **NO async in main thread** - egui update loop is synchronous, always use `std::thread::spawn` + Tokio runtime
- **Artwork URLs**: Replace `-large.jpg` with `-t500x500.jpg` for high quality
- **Track filtering**: Skip tracks where `streamable != true` or `stream_url` is None
- **Audio seeking**: Requires full reload at new position (rodio limitation, no pause/resume)
- **Texture loading**: Must happen in main thread via `ctx.load_texture()`
- **Cache workflow**: Check `CacheDB::is_cached()` first, then read filesystem
- **Volume**: NOT persisted, reverts on restart
- **Memory**: Explicitly `drop()` heavy resources (AudioPlayer, textures) to free memory

## Common Workflows

**Add playback control:**
1. Add `AudioCommand` enum variant
2. Handle in `AudioController` thread loop
3. Add public method to `AudioController`
4. Wire to UI in `ui_components/player.rs`

**Fetch API data:**
1. Define struct with `#[derive(Deserialize)]` in `models/`
2. Create async fetch in `api/` using `reqwest`
3. Add `Option<Receiver<T>>` to `MusicPlayerApp`
4. Spawn background thread, poll receiver in `update()`

**Add cache type:**
1. Create `get_*_cache_path()` in `utils/cache.rs`
2. Add save/load functions with SHA256 hashing
3. Include in cleanup iteration

**Add screen:**
1. Create `screens/new_screen.rs` with `render_*_view()`
2. Add variant to `MainTab` enum
3. Route in `ui_components/layout.rs::render_with_layout()`

## Testing
- Test binaries: `cargo run --release --bin play_history_test`
- No formal unit tests - relies on runtime testing and logs
- Shader converter: `cargo run --release --bin shadertoy_converter`
