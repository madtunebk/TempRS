# TempRS Refactoring Roadmap

**Generated:** 2025-12-08
**Analysis Scope:** Full codebase audit focusing on player_app.rs

## Executive Summary

The codebase analysis identified **150+ code quality issues** across severity levels:
- **CRITICAL**: 30+ unwrap/panic calls that can crash the app
- **HIGH**: 1 god object (160+ fields), 15 untracked threads, 29 excessive clones
- **MEDIUM**: 50+ dead code markers, 3 Arc<Mutex<f32>> anti-patterns
- **LOW**: Commented-out code, style issues

**Primary Focus**: `src/app/player_app.rs` is a 2,100-line monolith handling too many responsibilities.

---

## Phase 1: IMMEDIATE FIXES (Week 1-2) - CRITICAL

### 1.1 Replace Panic-Prone Unwraps
**Impact**: Prevents app crashes
**Files**: player_app.rs, utils/audio_controller.rs, utils/token_helper.rs, utils/artwork.rs

**Before:**
```rust
let rt = tokio::runtime::Runtime::new().unwrap();
```

**After:**
```rust
let rt = match tokio::runtime::Runtime::new() {
    Ok(r) => r,
    Err(e) => {
        log::error!("[Runtime] Failed to create: {}", e);
        self.toast_manager.show_error("Internal error");
        return;
    }
};
```

**Affected Locations** (30+ instances):
- `player_app.rs:831, 854, 884, 907, 1038, 1665, 1685, 1714, 1747, 1863, 1881, 1947`
- `data/home_data.rs:48, 101`
- `utils/token_helper.rs:125, 133`
- `utils/artwork.rs:262, 400`
- `utils/audio_controller.rs:54`
- `utils/audio_fft.rs:42, 59, 84, 110` (mutex unwraps)

**Deliverable**: Create `src/utils/error_handling.rs` with safe wrappers:
```rust
pub fn create_runtime() -> Result<Runtime, RuntimeError> { ... }
pub fn lock_or_log<T>(mutex: &Mutex<T>, ctx: &str) -> Option<MutexGuard<T>> { ... }
```

### 1.2 Track Thread Join Handles
**Impact**: Prevents thread leaks, enables graceful shutdown
**Files**: player_app.rs (15 instances)

**Before:**
```rust
std::thread::spawn(move || {
    // Work happens, but we can't wait for completion or detect panics
});
```

**After:**
```rust
// In MusicPlayerApp struct:
pub struct MusicPlayerApp {
    background_tasks: Vec<JoinHandle<()>>,
    // ...
}

// When spawning:
let handle = std::thread::spawn(move || { ... });
self.background_tasks.push(handle);

// In cleanup_and_exit():
for handle in self.background_tasks.drain(..) {
    if let Err(e) = handle.join() {
        log::error!("[Cleanup] Thread panicked: {:?}", e);
    }
}
```

**Deliverable**: Add thread lifecycle management to app shutdown.

---

## Phase 2: HIGH-PRIORITY REFACTORING (Week 3-6)

### 2.1 Split player_app.rs God Object
**Impact**: Improves maintainability, enables parallel development
**Effort**: 2-3 weeks

**Current Structure** (160+ fields, 2100 lines):
```rust
pub struct MusicPlayerApp {
    // Audio (15 fields)
    pub audio_controller: AudioController,
    pub bass_energy: Arc<Mutex<f32>>,
    pub is_playing: bool,
    // ... 12 more

    // OAuth (10 fields)
    pub oauth_manager: Option<OAuthManager>,
    pub is_authenticating: bool,
    // ... 8 more

    // UI State (20+ fields)
    pub selected_tab: MainTab,
    pub search_query: String,
    // ... 18 more

    // Content (40+ fields)
    pub likes_tracks: Vec<Track>,
    pub playlists: Vec<Playlist>,
    // ... 38 more

    // Channels/Receivers (30+ fields)
    pub artwork_rx: Option<Receiver<ColorImage>>,
    pub search_rx: Option<Receiver<SearchResults>>,
    // ... 28 more
}
```

**Target Structure**:
```rust
// src/state/mod.rs
pub struct AppState {
    pub audio: AudioState,
    pub auth: AuthState,
    pub ui: UIState,
    pub content: ContentState,
    pub background: BackgroundTasks,
}

// src/state/audio_state.rs (15 fields → isolated module)
pub struct AudioState {
    pub controller: AudioController,
    pub playback_queue: PlaybackQueue,
    pub bass_energy: Arc<AtomicU32>,  // Note: changed from Mutex<f32>
    pub mid_energy: Arc<AtomicU32>,
    pub high_energy: Arc<AtomicU32>,
    pub is_playing: bool,
    pub volume: f32,
    pub muted: bool,
    pub shuffle_mode: bool,
    pub repeat_mode: RepeatMode,
    // Current track info
    pub current_track_id: Option<u64>,
    pub current_title: String,
    pub current_artist: String,
    pub current_duration_ms: u64,
    pub track_start_time: Option<Instant>,
}

impl AudioState {
    pub fn play_track(&mut self, track: Track) -> Result<(), PlaybackError> { ... }
    pub fn toggle_playback(&mut self) { ... }
    pub fn set_volume(&mut self, vol: f32) { ... }
    pub fn toggle_mute(&mut self) { ... }
}

// src/state/auth_state.rs (10 fields → isolated module)
pub struct AuthState {
    pub oauth_manager: Option<OAuthManager>,
    pub is_authenticating: bool,
    pub user_username: Option<String>,
    pub user_avatar_url: Option<String>,
    pub user_avatar_texture: Option<TextureHandle>,
    pub token_check_interval: Duration,
    pub last_token_check: Option<Instant>,
}

impl AuthState {
    pub fn is_logged_in(&self) -> bool { ... }
    pub fn get_token(&self) -> Option<String> { ... }
    pub fn logout(&mut self) { ... }
}

// src/state/ui_state.rs (20 fields → isolated module)
pub struct UIState {
    pub selected_tab: MainTab,
    pub search_query: String,
    pub search_type: SearchType,
    pub search_expanded: bool,
    pub queue_collapsed: bool,
    pub show_volume_popup: bool,
    pub show_user_menu: bool,
    pub toast_manager: ToastManager,
}

// src/state/content_state.rs (40 fields → isolated module)
pub struct ContentState {
    pub likes: LikesState,
    pub playlists: PlaylistsState,
    pub suggestions: SuggestionsState,
    pub history: HistoryState,
    pub search: SearchState,
    pub home: HomeContent,
}

pub struct LikesState {
    pub tracks: Vec<Track>,
    pub user_tracks: Vec<Track>,
    pub liked_track_ids: HashSet<u64>,
    pub liked_playlist_ids: HashSet<u64>,
    pub page: usize,
    pub page_size: usize,
    pub loading: bool,
}

// src/state/background_tasks.rs (30+ receivers)
pub struct BackgroundTasks {
    tasks: Vec<JoinHandle<()>>,
    channels: ChannelRegistry,
}

pub struct ChannelRegistry {
    pub artwork_rx: Option<Receiver<ColorImage>>,
    pub search_rx: Option<Receiver<SearchResults>>,
    pub playlist_rx: Option<Receiver<Playlist>>,
    // ... all other receivers
}

impl BackgroundTasks {
    pub fn spawn<F, T>(&mut self, task: F) -> Receiver<T>
    where F: FnOnce() -> T + Send + 'static,
          T: Send + 'static
    { ... }

    pub fn cleanup(&mut self) { ... }
}
```

**Migration Strategy**:
1. Create new state modules (week 1)
2. Move fields one module at a time (week 2-3)
3. Update all references (week 4)
4. Remove old structure (week 5)
5. Test thoroughly (week 6)

### 2.2 Consolidate Duplicate Like/Unlike Logic
**Impact**: Reduces code by 100+ lines
**Files**: player_app.rs:817-920

**Current**: Two nearly identical functions
```rust
pub fn toggle_like(&mut self, track_id: u64) {
    // 50 lines
}

pub fn toggle_playlist_like(&mut self, playlist_id: u64) {
    // 50 lines - almost identical!
}
```

**Refactored** (`src/services/social.rs`):
```rust
pub enum LikeTarget {
    Track(u64),
    Playlist(u64),
}

pub struct SocialService {
    liked_tracks: HashSet<u64>,
    liked_playlists: HashSet<u64>,
}

impl SocialService {
    pub fn toggle_like(
        &mut self,
        target: LikeTarget,
        token: String,
        toast_mgr: &mut ToastManager,
    ) -> JoinHandle<Result<(), String>> {
        let (id, is_liked, item_type) = match target {
            LikeTarget::Track(id) => {
                let liked = self.liked_tracks.contains(&id);
                (id, liked, "track")
            }
            LikeTarget::Playlist(id) => {
                let liked = self.liked_playlists.contains(&id);
                (id, liked, "playlist")
            }
        };

        // Update local state optimistically
        match target {
            LikeTarget::Track(id) => {
                if is_liked {
                    self.liked_tracks.remove(&id);
                } else {
                    self.liked_tracks.insert(id);
                }
            }
            LikeTarget::Playlist(id) => {
                if is_liked {
                    self.liked_playlists.remove(&id);
                } else {
                    self.liked_playlists.insert(id);
                }
            }
        }

        // Show toast
        let msg = if is_liked {
            format!("Removed from Liked {}", item_type)
        } else {
            format!("Added to Liked {}", item_type)
        };
        toast_mgr.show_success(&msg);

        // Spawn API call
        spawn_api_task(move || Box::pin(async move {
            match (target, is_liked) {
                (LikeTarget::Track(id), false) =>
                    api::likes::like_track(&token, id).await,
                (LikeTarget::Track(id), true) =>
                    api::likes::unlike_track(&token, id).await,
                (LikeTarget::Playlist(id), false) =>
                    api::likes::like_playlist(&token, id).await,
                (LikeTarget::Playlist(id), true) =>
                    api::likes::unlike_playlist(&token, id).await,
            }
        }))
    }
}
```

**Deliverable**: Reduce 100 lines of duplication to 40 lines of shared logic.

### 2.3 Extract Common Thread-Spawn Pattern
**Impact**: Eliminates 12+ instances of boilerplate
**Files**: player_app.rs, utils/*, data/*

**Create** `src/utils/async_helper.rs`:
```rust
use std::future::Future;
use std::pin::Pin;
use std::thread::JoinHandle;

pub type AsyncTaskResult<T> = Result<T, String>;
pub type AsyncTask<T> = Pin<Box<dyn Future<Output = AsyncTaskResult<T>> + Send>>;

/// Spawns a background thread running a Tokio task
/// Returns JoinHandle for tracking and graceful shutdown
pub fn spawn_api_task<F, T>(
    task_factory: F
) -> JoinHandle<AsyncTaskResult<T>>
where
    F: FnOnce() -> AsyncTask<T> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to create runtime: {}", e)),
        };

        rt.block_on(task_factory())
    })
}

/// Spawns task and sends result to channel
pub fn spawn_and_send<F, T>(
    task: F,
    tx: std::sync::mpsc::Sender<AsyncTaskResult<T>>
) -> JoinHandle<()>
where
    F: FnOnce() -> AsyncTask<T> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        let result = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt.block_on(task()),
            Err(e) => Err(format!("Runtime error: {}", e)),
        };

        let _ = tx.send(result);
    })
}
```

**Usage Example**:
```rust
// Before (15+ instances like this):
std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match crate::api::likes::like_track(&token, track_id).await {
            Ok(_) => log::info!("Success"),
            Err(e) => log::error!("Failed: {}", e),
        }
    });
});

// After:
let handle = spawn_api_task(move || Box::pin(async move {
    api::likes::like_track(&token, track_id).await
        .map_err(|e| e.to_string())
}));

// Track handle for shutdown
self.background_tasks.push(handle);
```

### 2.4 Replace Arc<Mutex<f32>> with AtomicU32
**Impact**: Improves FFT performance, removes lock contention
**Files**: player_app.rs:84-86, utils/audio_fft.rs, utils/audio_controller.rs

**Before**:
```rust
pub bass_energy: Arc<Mutex<f32>>,
pub mid_energy: Arc<Mutex<f32>>,
pub high_energy: Arc<Mutex<f32>>,

// Usage (blocking):
*self.bass_energy.lock().unwrap() = value;
let bass = *self.bass_energy.lock().unwrap();
```

**After**:
```rust
use std::sync::atomic::{AtomicU32, Ordering};

pub bass_energy: Arc<AtomicU32>,
pub mid_energy: Arc<AtomicU32>,
pub high_energy: Arc<AtomicU32>,

// Helper functions for f32 <-> u32 conversion
pub fn store_f32(atomic: &AtomicU32, value: f32) {
    atomic.store(value.to_bits(), Ordering::Relaxed);
}

pub fn load_f32(atomic: &AtomicU32) -> f32 {
    f32::from_bits(atomic.load(Ordering::Relaxed))
}

// Usage (non-blocking):
store_f32(&self.bass_energy, value);
let bass = load_f32(&self.bass_energy);
```

**Deliverable**: Eliminate 15+ mutex lock unwraps, improve audio analysis performance.

### 2.5 Break Down Long Functions
**Impact**: Improves readability, testability
**Targets**:

| Function | Lines | Action |
|----------|-------|--------|
| `default()` | 202 | Extract to builder pattern |
| `update()` | 157 | Split into `update_splash()` + `update_main()` |
| `check_track_finished()` | 129 | Extract state machine to separate module |
| `check_home_updates()` | 96 | Split per content type |
| `play_track()` | 98 | Extract validation, loading, playback |

**Example** (check_track_finished):
```rust
// Before: 129 lines, 6 levels of nesting

// After: Split into state machine
enum TrackEndBehavior {
    RepeatOne,
    RepeatAll,
    PlayNext,
    StopPlayback,
    PlayRandom,
}

impl MusicPlayerApp {
    fn determine_end_behavior(&self) -> TrackEndBehavior { ... }

    fn check_track_finished(&mut self) {
        if !self.should_check_finish() { return; }

        match self.determine_end_behavior() {
            TrackEndBehavior::RepeatOne => self.replay_current(),
            TrackEndBehavior::RepeatAll => self.loop_to_start(),
            TrackEndBehavior::PlayNext => self.play_next(),
            TrackEndBehavior::PlayRandom => self.play_random_from_history(),
            TrackEndBehavior::StopPlayback => self.stop_playback(),
        }
    }

    fn should_check_finish(&self) -> bool { ... }
    fn replay_current(&mut self) { ... }
    fn loop_to_start(&mut self) { ... }
    fn play_random_from_history(&mut self) { ... }
}
```

---

## Phase 3: MEDIUM-PRIORITY IMPROVEMENTS (Week 7-10)

### 3.1 Audit Dead Code Markers
**Impact**: Clean up codebase, remove false warnings
**Effort**: 1 week

**Action Items**:
1. Review all 50+ `#[allow(dead_code)]` markers
2. Categorize:
   - **Intentional** (color palette, future features) → Document why
   - **Actually unused** → Delete
   - **False positive** → Fix usage or remove marker
3. Create style guide for when to use `#[allow(dead_code)]`

**Files**:
- `ui_components/colors.rs` - 20+ color constants (keep, document)
- `app_state.rs` - Remove markers, fix usage
- `utils/cache.rs` - Implement or delete functions
- `api/*` - Mark public API clearly
- `player_app.rs:678` - Delete `reset_player_state()` if unused

### 3.2 Reduce Clone Overuse
**Impact**: Performance improvement, clearer ownership
**Files**: player_app.rs (29 instances)

**Strategy**:
1. Use `&` references where possible
2. Use `Arc::clone()` for shared ownership
3. Use `Rc<RefCell<>>` for single-threaded shared mutation
4. Document when clones are necessary (thread boundaries)

**Example**:
```rust
// Before:
let track = track.clone();
std::thread::spawn(move || {
    process_track(&track);
});

// After:
let track = Arc::new(track);
std::thread::spawn({
    let track = Arc::clone(&track);
    move || {
        process_track(&track);
    }
});
```

### 3.3 Reduce Nesting Depth
**Impact**: Readability
**Files**: player_app.rs:1224-1350

**Technique**: Early returns
```rust
// Before (6 levels):
if is_playing && is_finished {
    if current_track.is_some() {
        if track_start_time.is_some() {
            if let Some(start) = track_start_time {
                if start.elapsed() >= Duration::from_secs(1) {
                    // Actual logic here
                }
            }
        }
    }
}

// After (2 levels):
if !is_playing || !is_finished { return; }
if current_track.is_none() { return; }
if track_start_time.is_none() { return; }

let start = track_start_time.unwrap();
if start.elapsed() < Duration::from_secs(1) { return; }

// Actual logic here
```

### 3.4 Implement Shared Tokio Runtime
**Impact**: Performance, resource efficiency
**Files**: Create `src/runtime.rs`

**Current**: Creating runtime 12+ times per session
**Target**: One global runtime

```rust
// src/runtime.rs
use tokio::runtime::Runtime;
use once_cell::sync::Lazy;

static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Runtime::new().expect("Failed to create Tokio runtime")
});

pub fn runtime() -> &'static Runtime {
    &RUNTIME
}

// Usage:
runtime().spawn(async {
    // Work
});

runtime().block_on(async {
    // Blocking work
});
```

---

## Phase 4: LOW-PRIORITY POLISH (Week 11+)

### 4.1 Remove Commented-Out Code
**Files**: player_app.rs:213-219, others

### 4.2 Extract Magic Numbers to Constants
**Example**:
```rust
// Before:
if tracks.len() < 6 { ... }
self.suggestions_page_size = 12;

// After:
const HOME_RECOMMENDATIONS_COUNT: usize = 6;
const SUGGESTIONS_PAGE_SIZE: usize = 12;

if tracks.len() < HOME_RECOMMENDATIONS_COUNT { ... }
self.suggestions_page_size = SUGGESTIONS_PAGE_SIZE;
```

### 4.3 Add Comprehensive Error Types
**Create** `src/errors.rs`:
```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Playback error: {0}")]
    Playback(String),

    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type AppResult<T> = Result<T, AppError>;
```

---

## Testing Strategy

### Phase 1 Tests
- [ ] Runtime creation error handling
- [ ] Thread panic recovery
- [ ] Graceful shutdown with active threads

### Phase 2 Tests
- [ ] State module isolation (unit tests)
- [ ] Like/unlike consolidation (integration)
- [ ] Async helper correctness
- [ ] AtomicU32 FFT values (performance benchmark)

### Phase 3 Tests
- [ ] Dead code removal doesn't break features
- [ ] Clone reduction maintains correctness
- [ ] Shared runtime performance improvement

---

## Success Metrics

### Code Quality
- ✅ Zero unwraps in user-facing paths
- ✅ All threads tracked
- ✅ No functions > 100 lines
- ✅ No structs > 30 fields
- ✅ Clippy clean (zero warnings)

### Performance
- ✅ 30% reduction in thread creation overhead
- ✅ FFT processing latency < 1ms (atomic vs mutex)
- ✅ App startup time < 500ms

### Maintainability
- ✅ 50% reduction in player_app.rs size (2100 → ~1000 lines)
- ✅ State modules < 300 lines each
- ✅ Code duplication < 5% (from ~15%)

---

## Risk Mitigation

1. **Regression Testing**: Comprehensive test suite before refactoring
2. **Feature Flags**: Enable/disable refactored code paths
3. **Incremental Migration**: One module at a time
4. **Code Reviews**: Two-person review for critical changes
5. **Rollback Plan**: Git branches for each phase

---

## Timeline Summary

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| Phase 1 | 2 weeks | Safe unwraps, thread tracking |
| Phase 2 | 4 weeks | State split, consolidation, helpers |
| Phase 3 | 4 weeks | Dead code cleanup, optimization |
| Phase 4 | Ongoing | Polish, documentation |

**Total Estimated Effort**: 10-12 weeks for core refactoring
