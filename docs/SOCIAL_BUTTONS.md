# Social Buttons Implementation

## Overview

Added like and share buttons to the player bar controls, positioned between the progress bar and volume controls.

## UI Layout

Player bar now has 4 sections (left to right):
1. **Playback controls** (200px): Shuffle, Prev, Stop, Play/Pause, Next, Repeat
2. **Progress bar** (55% elastic, min 280px): Current position / Duration
3. **Social buttons** (100px): Like (💜/🤍), Share (🔗) - **NEW**
4. **Volume controls** (140px): Mute, Volume slider

## Features

### Like Button
- **Visual States**:
  - Liked: Purple heart (💜) with purple background (#8040100)
  - Not liked: White heart (🤍) with transparent background
- **Behavior**:
  - Instant UI update (optimistic)
  - Background API call to PUT/DELETE `/me/likes/tracks/{id}`
  - State tracked in `liked_track_ids` HashSet
  - Populated from liked tracks fetch
- **API Endpoints**: 
  - `like_track(token, track_id)` - PUT request
  - `unlike_track(token, track_id)` - DELETE request

### Share Button
- **Icon**: Link emoji (🔗)
- **Behavior**:
  - Copies SoundCloud URL to clipboard: `https://soundcloud.com/track/{id}`
  - Uses `arboard` crate for cross-platform clipboard access
  - Logs success/failure
  - TODO: Add toast notification for user feedback

## Implementation Details

### Files Modified

1. **`src/ui_components/player.rs`**:
   - Added `render_social_buttons()` function
   - Inserted social buttons section in main layout
   - 40x40px circular buttons matching player bar style

2. **`src/app/player_app.rs`**:
   - Added `liked_track_ids: HashSet<u64>` field
   - Populated HashSet when liked tracks received
   - Added methods:
     - `is_current_track_liked()` - Check if current track is in liked set
     - `toggle_current_track_like()` - Toggle like state with API call
     - `share_current_track()` - Copy track URL to clipboard

3. **`src/api/likes.rs`**:
   - Added `like_track(token, track_id)` - PUT endpoint
   - Added `unlike_track(token, track_id)` - DELETE endpoint

4. **`Cargo.toml`**:
   - Added `arboard = "3.4"` for clipboard functionality

### State Management

- **Liked state**: Tracked in `liked_track_ids` HashSet for O(1) lookup
- **Optimistic updates**: UI updates immediately, API called in background
- **Sync on fetch**: HashSet rebuilt when liked tracks fetched
- **No persistence**: Liked state loaded from API on app start

### Background Task Pattern

Like/unlike operations use standard background task pattern:
```rust
std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match api_call().await {
            Ok(_) => log success,
            Err(e) => log error,
        }
    });
});
```

## Future Enhancements

1. **Share button improvements**:
   - Add toast notification on copy success
   - Show share menu with options (copy link, open in browser)
   - Add track URL to track metadata for accurate sharing

2. **Like button improvements**:
   - Animation on like/unlike
   - Error handling with rollback on API failure
   - Sync liked state across screens immediately

3. **Additional social features**:
   - Repost button
   - Add to playlist quick action
   - Download track (if available)

## Testing

Compile: `cargo build --release`
Run: `cargo run --release --bin TempRS`

**Test scenarios**:
1. Click like button - should toggle heart color and background
2. Click share button - track URL should be in clipboard
3. Restart app - liked state should persist (loaded from API)
4. Unlike from Likes screen - player bar should update
