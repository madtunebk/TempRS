# Code Quality Analysis Meta Prompt for TempRS

## Purpose
This meta prompt defines a systematic approach to analyze Rust codebases for code quality issues, refactoring opportunities, and architectural improvements.

## Analysis Dimensions

### 1. Code Health Metrics
- **Dead Code Detection**: Identify unused functions, variables, imports, and code marked with `#[allow(dead_code)]`
- **Deprecated Patterns**: Find commented-out code, stale TODOs, and outdated implementations
- **Code Coverage**: Determine which parts of the codebase lack tests or documentation

### 2. Safety & Reliability
- **Panic Sources**: Scan for `.unwrap()`, `.expect()`, and potential panics
- **Error Handling**: Evaluate proper use of `Result<T, E>` vs panic-prone patterns
- **Unsafe Code**: Identify and justify `unsafe` blocks
- **Resource Management**: Check for proper cleanup, leaked threads, unclosed connections
- **Concurrency Safety**: Analyze Arc/Mutex usage, thread spawns, race conditions

### 3. Code Complexity
- **Function Length**: Flag functions exceeding 50-100 lines
- **Cyclomatic Complexity**: Measure branching (if/match statements)
- **Nesting Depth**: Identify deep nesting (>4 levels)
- **Parameter Count**: Functions with >5 parameters
- **Struct Field Count**: Large structs (>20 fields) violating Single Responsibility

### 4. Performance Anti-Patterns
- **Clone Overuse**: Excessive `.clone()` calls instead of borrowing
- **Arc<Mutex<Copy>>**: Should use atomics for simple types
- **Repeated Runtime Creation**: Creating Tokio runtimes repeatedly
- **Inefficient String Operations**: Unnecessary allocations
- **Synchronous Operations in Async**: Blocking in async contexts

### 5. Duplication & Maintainability
- **Code Duplication**: Similar logic across multiple functions/files
- **Copy-Paste Programming**: Near-identical code blocks with minor variations
- **Missing Abstractions**: Repeated patterns that should be extracted
- **Magic Numbers**: Hardcoded values without named constants

### 6. Architecture & Design
- **God Objects**: Structs/modules doing too much
- **Tight Coupling**: Hard dependencies preventing modularity
- **Separation of Concerns**: Business logic mixed with UI/IO
- **API Design**: Public interfaces that could be simplified

## Analysis Methodology

### Phase 1: Automated Scanning
```bash
# Use cargo tooling
cargo clippy --all-targets --all-features
cargo outdated
cargo audit
cargo tree --duplicates
```

### Phase 2: Manual Review Checklist
For each source file:
1. Count function lines (target: <50)
2. Identify `.unwrap()` and `.expect()` calls
3. Check for `#[allow(dead_code)]` markers
4. Look for thread::spawn without JoinHandle tracking
5. Identify duplicated code patterns
6. Review struct field counts
7. Analyze error propagation

### Phase 3: Pattern Recognition
Identify common anti-patterns:
- Thread spawn + Tokio runtime creation (should use shared runtime)
- Channel-based async (tx/rx pairs) without timeout handling
- Cloning for thread safety (consider Arc/Rc)
- Mutex locks without poisoning checks
- State machines with deep nesting (refactor to match/enum)

### Phase 4: Prioritization
Classify findings by severity:
- **CRITICAL**: Panics, unsafe operations, thread leaks
- **HIGH**: God objects, major duplication, performance issues
- **MEDIUM**: Code smells, minor duplication, complexity
- **LOW**: Style issues, minor optimizations

## Reporting Format

### Summary Table
| Category | Count | Severity | Impact | Files Affected |
|----------|-------|----------|--------|----------------|
| Unwrap calls | X | CRITICAL | App crashes | file1.rs, file2.rs |
| Large structs | X | HIGH | Maintainability | player_app.rs |
| Clone overuse | X | MEDIUM | Performance | multiple |

### Detailed Findings
For each issue:
1. **Location**: File path + line numbers
2. **Severity**: CRITICAL / HIGH / MEDIUM / LOW
3. **Description**: What the issue is
4. **Impact**: Why it matters
5. **Example Code**: Show the problematic pattern
6. **Suggested Fix**: Provide refactoring approach with code example

### Refactoring Roadmap
Prioritized action plan:
1. **Immediate** (Critical fixes)
2. **Short-term** (High-priority improvements)
3. **Medium-term** (Medium-priority refactoring)
4. **Long-term** (Low-priority optimizations)

## player_app.rs Specific Analysis

### Structural Issues
- **Monolithic struct** (160+ fields): Split into domain-focused modules
  - `AudioState` - playback, volume, FFT data
  - `UIState` - selected tab, search query, UI flags
  - `AuthState` - OAuth, token management, user info
  - `ContentState` - playlists, likes, suggestions, history

### Function Extraction Targets
Long functions in player_app.rs that should be extracted:

1. **UI Components** → Move to `src/ui_components/`:
   - Volume controls
   - Queue UI
   - Player controls rendering
   - Toast notifications

2. **Screen Logic** → Move to `src/screens/`:
   - Search result handling
   - Playlist loading logic
   - Likes/history pagination

3. **Utilities** → Move to `src/utils/`:
   - Artwork loading/caching
   - Token validation
   - Background task spawning
   - Channel-based async helpers

4. **State Management** → New `src/state/`:
   - `PlaybackState` - current track, position, playing flag
   - `SearchState` - query, results, pagination
   - `LibraryState` - likes, playlists, history

### Code Movement Examples

#### Extract Volume Management
```rust
// Before (in player_app.rs):
pub fn set_volume(&mut self, volume: f32) {
    self.volume = volume.clamp(0.0, 1.0);
    self.audio_controller.set_volume(self.volume);
    self.save_playback_config();
}

// After (new file src/state/audio_state.rs):
pub struct AudioState {
    pub volume: f32,
    pub muted: bool,
    pub controller: AudioController,
}
impl AudioState {
    pub fn set_volume(&mut self, volume: f32) { ... }
    pub fn toggle_mute(&mut self) { ... }
}
```

#### Extract Like/Unlike Logic
```rust
// Before: Duplicated in toggle_like() and toggle_playlist_like()

// After (new file src/services/social.rs):
pub enum SocialItem {
    Track(u64),
    Playlist(u64),
}
pub async fn toggle_like(
    token: &str,
    item: SocialItem,
    is_liked: bool
) -> Result<(), ApiError> {
    match (item, is_liked) {
        (SocialItem::Track(id), true) => unlike_track(token, id).await,
        (SocialItem::Track(id), false) => like_track(token, id).await,
        (SocialItem::Playlist(id), true) => unlike_playlist(token, id).await,
        (SocialItem::Playlist(id), false) => like_playlist(token, id).await,
    }
}
```

#### Extract Background Task Helper
```rust
// Before: Repeated 15+ times
std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { ... });
});

// After (new file src/utils/async_helper.rs):
pub fn spawn_api_task<F, T>(
    task: F
) -> JoinHandle<Result<T, String>>
where
    F: FnOnce() -> Pin<Box<dyn Future<Output = Result<T, String>>>> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to create runtime: {}", e)),
        };
        rt.block_on(task())
    })
}

// Usage:
let handle = spawn_api_task(move || Box::pin(async move {
    api::likes::like_track(&token, track_id).await
        .map_err(|e| e.to_string())
}));
```

## Suggested File Structure After Refactoring

```
src/
├── app/
│   ├── mod.rs                    # Re-export main app struct
│   ├── player_app.rs            # MUCH SMALLER (30-50 fields max)
│   ├── queue.rs                 # ✓ Already separate
│   └── playlists.rs             # ✓ Already separate
│
├── state/                        # NEW: Extracted state modules
│   ├── mod.rs
│   ├── audio_state.rs           # Playback, volume, FFT
│   ├── ui_state.rs              # Selected tab, search, flags
│   ├── auth_state.rs            # OAuth, tokens, user info
│   └── content_state.rs         # Likes, playlists, suggestions
│
├── services/                     # NEW: Business logic
│   ├── mod.rs
│   ├── social.rs                # Like/unlike consolidated
│   ├── playback_manager.rs     # Play/pause/skip logic
│   └── content_loader.rs       # Fetch playlists/likes/suggestions
│
├── ui_components/               # ✓ Already exists
│   ├── player.rs               # ✓ Already separate
│   ├── volume_controls.rs      # NEW: Extract from player_app
│   └── ...
│
├── screens/                     # ✓ Already exists
│   ├── home/
│   ├── search/
│   └── ...
│
└── utils/
    ├── async_helper.rs          # NEW: spawn_api_task()
    ├── error_handling.rs        # NEW: Custom error types
    └── ...
```

## Code Quality Goals

### Target Metrics (After Refactoring)
- ✅ No functions > 100 lines
- ✅ No structs > 30 fields
- ✅ Zero `.unwrap()` in user-facing code paths
- ✅ All threads tracked with JoinHandles
- ✅ <10 instances of `#[allow(dead_code)]`
- ✅ All API errors propagated, not swallowed
- ✅ Shared Tokio runtime (no repeated creation)

### Testing Strategy
- Unit tests for extracted modules
- Integration tests for API services
- Property tests for state machines
- Clippy compliance (zero warnings)

## Tools & Commands

### Analysis
```bash
# Find all unwraps
rg "\.unwrap\(\)" --type rust src/

# Find dead code markers
rg "#\[allow\(dead_code\)\]" --type rust src/

# Count function lines
tokei src/ --files

# Find long functions
rg "^pub fn|^fn" -A 100 src/ | rg "^}" | wc -l
```

### Refactoring Safety
```bash
# Before refactoring
cargo test
cargo clippy -- -D warnings
cargo build --release

# After each change
cargo check
cargo test
```

## Conclusion

This meta prompt provides a systematic framework for analyzing and improving code quality in Rust projects. Apply iteratively, prioritize critical issues, and track progress with metrics.
