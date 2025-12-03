# Playback History Feature

## Overview
TempRS now uses a **local SQLite database** to track playback history instead of relying on the SoundCloud API. This provides more accurate and reliable "Recently Played" data on the Home screen.

## Implementation

### Database Location
- **Path**: `~/.config/TempRS/playback_history.db` (Linux) or equivalent on other platforms
- **Engine**: SQLite via rusqlite
- **Schema**: Single table `playback_history` with track metadata and play timestamps

### How It Works

1. **Recording Playback**
   - When a track starts playing (in `play_track()`), a `PlaybackRecord` is created
   - The record is saved to the database in a background thread (non-blocking)
   - Each track is keyed by `track_id` (PRIMARY KEY), so replays update the timestamp

2. **Retrieving Recently Played**
   - Home screen calls `fetch_recently_played_async()` which queries the local database
   - No API call needed! Fetches last 6 tracks ordered by `played_at DESC`
   - Converts `PlaybackRecord` to `Track` for display compatibility

3. **Recommendations (API-based)**
   - Still uses SoundCloud API to fetch related tracks based on recently played
   - Takes the first track from local history and calls `fetch_related_tracks()`
   - This provides the "More of what you like" section

### Benefits

✅ **Accurate**: Tracks exactly what you played in TempRS, not other clients  
✅ **Fast**: No network latency - instant database query  
✅ **Private**: Your play history stays local, not sent to SoundCloud  
✅ **Reliable**: Works even if SoundCloud API changes their activity endpoint  
✅ **Persistent**: History survives app restarts and token refreshes  

### Database Schema

```sql
CREATE TABLE playback_history (
    track_id INTEGER PRIMARY KEY,      -- SoundCloud track ID
    title TEXT NOT NULL,               -- Track title
    artist TEXT NOT NULL,              -- Artist/uploader name
    artwork_url TEXT,                  -- Artwork URL (cached separately)
    duration INTEGER NOT NULL,         -- Track duration in milliseconds
    genre TEXT,                        -- Genre tag
    stream_url TEXT,                   -- Stream URL for playback
    played_at INTEGER NOT NULL         -- Unix timestamp of last play
);

CREATE INDEX idx_played_at ON playback_history(played_at DESC);
```

### API Changes

**Modified Files:**
- `src/utils/playback_history.rs` - New database module
- `src/app/home.rs` - Changed `fetch_recently_played_async()` to query local DB
- `src/app/player_app.rs` - Added `record_playback_history()` method

**Home Screen Workflow:**
1. **Recently Played**: Local database (6 tracks)
2. **Recommendations**: SoundCloud API based on first recently played track (6-12 tracks)

### Maintenance

The database grows with each unique track played. Optional cleanup:
- `cleanup_old_records(days)` - Remove entries older than N days
- `clear_all()` - Wipe entire history
- Currently no automatic cleanup (manual implementation needed if desired)

### Migration Notes

- **No breaking changes**: Existing installations will create the database on first track play
- **First-time users**: Empty database shows fallback popular tracks (unchanged behavior)
- **Database size**: ~500 bytes per track, negligible storage impact even with 1000+ tracks
