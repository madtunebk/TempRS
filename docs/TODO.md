# TempRS - TODO List

## COMPLETED WORK - December 5, 2025 ✅

### UI/UX Improvements ✅
- ✅ Social buttons moved to top-left corner of artwork
- ✅ Share button removed from artwork (exclusive to player bar)
- ✅ Fixed widget ID clashes using artwork position for unique IDs
- ✅ Consistent headers across History, Suggestions, Likes, Playlists (24px, white, bold)
- ✅ Removed duplicate headers from screens
- ✅ Red error messages eliminated (widget ID conflicts resolved)

### Startup & Performance ✅
- ✅ Added 2-second minimum splash screen duration
- ✅ Smooth window initialization (prevents weird startup behavior)
- ✅ Active timer checking with repaint requests
- ✅ Debug logging for elapsed time tracking

### Credential Management ✅
- ✅ Switched to .env file system (dotenvy)
- ✅ build.rs loads credentials at compile time
- ✅ .env.example template created
- ✅ Removed hardcoded credentials from source
- ✅ Removed obsolete credentials.example.rs and CREDENTIALS_SETUP.md

### Documentation Cleanup ✅
- ✅ Organized all .md files into docs/ folder
- ✅ Removed 10 outdated feature documentation files
- ✅ Kept 9 active/relevant docs
- ✅ Clean repository structure

### Repository Maintenance ✅
- ✅ Updated .gitignore (commit.sh, test_play_history.sh, tools.txt)
- ✅ Removed local scripts from git tracking
- ✅ Pushed cleanup to both remotes (origin + github)

## COMPLETED WORK - December 2, 2025 ✅

### Volume Control Enhancement ✅
- ✅ Vertical popup slider (140px tall)
- ✅ Click speaker icon to show/hide popup
- ✅ Right-click speaker to mute/unmute
- ✅ Shadow layers and orange accent styling
- ✅ Auto-unmute when adjusting volume while muted

### Graceful Shutdown System ✅
- ✅ Proper cleanup of audio resources
- ✅ Saves playback settings on exit
- ✅ Clears receivers, textures, and caches
- ✅ No confirmation dialog (direct cleanup)
- ✅ Logs each cleanup step for debugging

### Audio Sync Improvements ✅
- ✅ Added buffering state tracking (prevents premature "finished" detection)
- ✅ 5-second timeout detection for stuck streams
- ✅ Buffer management: 5MB limit with 2MB trim when exceeded
- ✅ Fixed rare endless stuck/choppy audio issues
- ✅ Reverted incremental decoding (caused choppy audio)

### Like/Unlike Functionality ✅
- ✅ Track like/unlike from Likes screen and player footer
- ✅ Playlist like/unlike from Playlists tab
- ✅ Optimistic UI updates with background API sync
- ✅ Heart icons: ❤ (liked) / 💔 (unliked)
- ✅ Red hover effect on unlike buttons
- ✅ API integration: `like_track()`, `unlike_track()`, `like_playlist()`, `unlike_playlist()`
- ✅ Toast notifications for like/unlike actions

### UI/UX Polish ✅
- ✅ Removed duplicate badges from track cards
- ✅ Character encoding fixes (× → x/X)
- ✅ Toast notifications: text-only (no emojis)
- ✅ Broken heart icon for unliked state
- ✅ Clean heart button placement on playlist cards

### Git Repository ✅
- ✅ Remote updated: `ssh://gitea@gitea.home.cornfield/nobus/TempRS.git`
- ✅ All changes committed and pushed
- ✅ No more custom ports to manage

## PREVIOUS COMPLETIONS - December 1, 2025 ✅
- ✅ **Like state on startup**: Added `fetch_liked_track_ids_only()` method
  - Fetches liked track IDs immediately after authentication
  - Populates `liked_track_ids` HashSet without waiting for Likes tab visit
  - Called in both startup token check AND new authentication flows
- ✅ **File renamed for clarity**: `playlists.rs` → `user_playlists.rs`
  - Function: `render_playlists_view()` → `render_user_playlists_view()`
  - Prevents confusion between "user playlists" (SoundCloud) and "playback queue"
- ✅ **Centered layout**: Likes and User Playlists screens now use `calculate_grid_layout()`
  - Matches Suggestions screen layout pattern
  - Properly centered content with consistent padding
  - Improved title size (24.0 with strong weight, matching Suggestions)
  - Centered empty states with icons and descriptions
- ✅ **Like/Unlike functionality**: Working across multiple views
- ✅ **Share functionality**: Working - copies track URL to clipboard
- ✅ **API integration**: `api/likes.rs` has like_track() and unlike_track() methods
- ✅ **State management**: `liked_track_ids: HashSet<u64>` in MusicPlayerApp
- ✅ **Visual feedback**: Orange filled heart for liked, gray outline for not liked

### Where Social Buttons Appear:
- ✅ **Home tab (Now Playing)**: Main "Now Playing" view shows social buttons below artist name
  - Located in `src/screens/home/mod.rs` in `render_now_playing_view()`
  - Shows when a track is playing on the Home tab
- ✅ **Now Playing tab sidebar**: Track metadata sidebar on right
  - Located in `src/ui_components/track_metadata.rs`
  - Shows when viewing the Now Playing tab with a playlist loaded
- ⚠️ **NOT in User Playlists tab**: That's a list of playlists, not track playback
- ⚠️ **NOT in track grid cards**: Only in detail views, not grid/list items

### Remaining Polish (Non-Critical):
- [x] **Toast notifications**: Success/error messages for social actions implemented
  - Like/unlike confirmation ✅
  - Share success feedback ✅
  - Text-only messages (no emojis) ✅
- [ ] **Error handling UI**: Show user-friendly error messages when API calls fail
- [ ] **Cleanup unused code**: Remove unused `render_social_buttons()` in player.rs if not needed

### Testing Checklist:
- [x] Liked tracks load on app startup
- [x] Heart icon shows correct state (filled/outline)
- [x] Like/unlike updates HashSet immediately (optimistic)
- [x] Share copies URL to clipboard
- [x] Toast notification system working
- [x] Playlist like/unlike functionality working

## Today's Tasks (November 29, 2025) - UPDATED December 1, 2025

### 1. History Screen (Dedicated View) ✅ COMPLETED
- [x] Create `src/screens/history.rs` module
- [x] Add "History" tab to MainTab enum
- [x] Display all playback history from database
- [x] Show: track title, artist, genre
- [x] Sort by most recent first
- [x] Add pagination/infinite scroll for large histories
- [x] Click track to play
- [x] Grid or list view with artwork

### 2. User Playlists Screen ✅ COMPLETED
- [x] Create `src/screens/playlists.rs` module
- [x] Add "Playlists" tab to MainTab enum
- [x] Fetch user's playlists from SoundCloud API endpoint
  - `GET /me/playlists` with OAuth token
- [x] Display playlist grid with artwork, title, track count
- [x] Click playlist to view tracks
- [x] Playlist detail view (similar to search playlist view)
- [x] Play entire playlist
- [x] Show created/liked playlists

### 3. Social Interaction Buttons ✅ PARTIALLY COMPLETED
#### Like/Unlike Tracks ✅ COMPLETED
- [x] Add heart/like button to Now Playing screen
- [x] Add heart/like button to track cards (multiple locations)
- [x] API endpoint: `PUT /me/favorites/{track_id}` (like)
- [x] API endpoint: `DELETE /me/favorites/{track_id}` (unlike)
- [x] Update UI state immediately (optimistic update)
- [x] Show liked state with filled vs outline heart icon

#### Share Button ✅ COMPLETED
- [x] Add share button to Now Playing screen
- [x] Copy track URL to clipboard functionality
- [x] Implemented in multiple locations (home, player, track metadata)

#### Add to Playlist ⏳ TODO
- [ ] Add "Add to Playlist" button (+ icon)
- [ ] Show modal/dropdown with user's playlists
- [ ] API endpoint: `PUT /playlists/{playlist_id}/tracks?track_id={track_id}`
- [ ] Success/error notification
- [ ] Allow creating new playlist from modal

#### Playlist Management ⏳ TODO
- [ ] Create Playlist modal/dialog
  - Input: title, description (optional)
  - API: `POST /playlists` with JSON body
- [ ] Delete Playlist confirmation dialog
  - API: `DELETE /playlists/{playlist_id}`
  - Remove from UI after deletion
- [ ] Edit playlist (future enhancement)

### 4. Related Content Screen (Dedicated View) ⏳ TODO
- [ ] Create `src/screens/related.rs` module (or expand history screen)
- [ ] Show "More like this" section
- [ ] Based on currently playing track or selected track
- [ ] Display related tracks in grid
- [ ] Use existing `fetch_related_tracks()` API function
- [ ] Click to play or add to queue

## Implementation Notes

### API Endpoints to Implement
```rust
// User Playlists
GET /me/playlists
GET /playlists/{id}
POST /playlists (body: {title, description, sharing})
DELETE /playlists/{id}
PUT /playlists/{id} (edit)

// Favorites/Likes
GET /me/favorites (track IDs)
PUT /me/favorites/{track_id}
DELETE /me/favorites/{track_id}

// Playlist Tracks
GET /playlists/{id}/tracks
PUT /playlists/{id}/tracks?track_id={track_id}
DELETE /playlists/{id}/tracks/{track_id}
```

### UI Components to Create
- Like button (heart icon - outline/filled states)
- Add to playlist button (+ icon)
- Playlist selector modal
- Create playlist dialog
- Delete confirmation dialog
- History list/grid view
- User playlists grid

### Data Structures Needed
```rust
pub struct Playlist {
    pub id: u64,
    pub title: String,
    pub description: Option<String>,
    pub artwork_url: Option<String>,
    pub track_count: u64,
    pub created_at: String,
    pub user: User,
}

pub struct UserPlaylistsResponse {
    pub collection: Vec<Playlist>,
    pub next_href: Option<String>,
}
```

### State Management
- Add `liked_tracks: HashSet<u64>` to MusicPlayerApp
- Add `user_playlists: Vec<Playlist>` to MusicPlayerApp
- Add `show_playlist_modal: bool` for add-to-playlist UI
- Add `show_create_playlist_dialog: bool`

## Priority Order
1. **User Playlists Screen** (Core functionality)
2. **Like/Unlike Buttons** (Most used social feature)
3. **Add to Playlist** (Playlist interaction)
4. **Playlist Management** (Create/Delete)
5. **History Screen** (Enhanced view of existing data)
6. **Related Content Screen** (Bonus feature)

## Estimated Time
- User Playlists Screen: 2-3 hours
- Social Buttons (Like): 1-2 hours
- Add to Playlist: 1-2 hours
- Playlist Management: 1-2 hours
- History Screen: 1 hour
- Related Content: 30 mins (already have API)

**Total: ~8-10 hours of work**

---

## Future Ideas - Extract Reusable Crates 📦

Potential standalone crates to extract from TempRS:

1. **`rodio-streaming`** - Progressive MP3 streaming with seeking
   - StreamingSource iterator for rodio
   - Buffer management (5MB limit with trimming)
   - Byte offset seeking support
   - CDN retry logic with exponential backoff
   - Use case: Any audio player needing progressive streaming

2. **`audio-fft-analyzer`** - Real-time FFT audio analysis
   - Dual-channel architecture (download + playback streams)
   - Bass/mid/high frequency extraction
   - Thread-safe sample processing with rustfft
   - Use case: Visualizers, DJ apps, audio analysis tools

3. **`soundcloud-rs`** - SoundCloud API client
   - OAuth with PKCE flow
   - Tracks, playlists, likes, search, users endpoints
   - Token refresh and retry logic
   - Use case: Any SoundCloud integration

4. **`secure-token-store`** - Encrypted credential storage
   - AES-256-GCM encryption with machine fingerprint keys
   - SQLite backend for persistence
   - Cross-platform machine fingerprinting
   - Use case: Desktop apps needing secure token storage

5. **`hybrid-cache`** - Filesystem + SQLite caching system
   - SHA256-based file naming
   - Metadata tracking with SQLite
   - Auto-cleanup by age and size limits
   - Placeholder support to prevent retry loops
   - Use case: Any app caching remote resources (images, audio, etc.)

Benefits: Portfolio boost, help other Rust devs, reuse in future projects

