# Cache Database Integration

## Overview

TempRS now uses a **hybrid caching system** combining filesystem storage with SQLite metadata tracking for optimal performance and reliability.

## Architecture

### Two-Layer Cache System

1. **Filesystem Layer** (`~/.cache/TempRS/`)
   - Stores actual file data (images, audio chunks)
   - Organized by type: `artwork/`, `sidebar_artwork/`, `audio/`
   - Files named using SHA256 hash of URL: `{hash}.jpg`

2. **Database Layer** (`~/.cache/TempRS/cache.db`)
   - SQLite database tracking cache metadata
   - Fast lookups without filesystem stat calls
   - Enables advanced features: cleanup, statistics, placeholder tracking

### Database Schema

```sql
CREATE TABLE cache_entries (
    url TEXT NOT NULL,              -- Original URL
    cache_type TEXT NOT NULL,       -- 'artwork', 'sidebar_artwork', etc.
    file_hash TEXT NOT NULL,        -- SHA256 hash used for filename
    cached_at INTEGER NOT NULL,     -- Unix timestamp
    file_size INTEGER NOT NULL,     -- File size in bytes
    is_placeholder INTEGER NOT NULL, -- 1 if placeholder, 0 if real data
    PRIMARY KEY (url, cache_type)
);
CREATE INDEX idx_cache_type ON cache_entries(cache_type);
```

## Key Features

### 1. Placeholder Tracking

The `is_placeholder` flag prevents retry loops for failed URLs:

- **Real artwork**: `is_placeholder = false`
- **Failed fetch (no_artwork.png)**: `is_placeholder = true`

When loading from cache, the app knows immediately if it's a placeholder without network requests.

### 2. Fast Cache Lookups

Before: `fs::metadata()` call for every cache check (slow)
After: Single SQL query checking database index (fast)

```rust
// Check if cached (database lookup)
if let Ok(db) = CacheDB::new() {
    if !db.is_cached(url, "artwork") {
        return None; // Skip filesystem check
    }
}

// Only read file if database confirms it exists
fs::read(path).ok()
```

### 3. Automatic Cleanup

On app startup, cache entries older than 7 days are automatically removed:

```rust
impl Default for MusicPlayerApp {
    fn default() -> Self {
        // Clean up old cache entries on startup
        let _ = crate::utils::cache::cleanup_old_cache_db(7);
        // ...
    }
}
```

This keeps cache size manageable without manual intervention.

### 4. Cache Statistics (Future Use)

Database enables easy statistics:

```rust
let stats = db.get_stats();
println!("Artwork: {} files", stats.artwork_count);
println!("Sidebar: {} files", stats.sidebar_count);
println!("Placeholders: {}", stats.placeholder_count);
println!("Total size: {} bytes", stats.total_size);
```

## API Changes

### Cache Save Functions

**Before:**
```rust
save_artwork_cache(url, data) -> Result<(), std::io::Error>
save_sidebar_artwork_cache(url, data) -> Result<(), std::io::Error>
```

**After:**
```rust
save_artwork_cache(url, data, is_placeholder) -> Result<(), std::io::Error>
save_sidebar_artwork_cache(url, data, is_placeholder) -> Result<(), std::io::Error>
```

**Usage:**
```rust
// Save real artwork
save_artwork_cache(&url, &bytes, false)?;

// Save placeholder
save_artwork_cache(&url, no_artwork_bytes, true)?;
```

### Cache Load Functions

No signature changes - database check happens transparently:

```rust
load_artwork_cache(url) -> Option<Vec<u8>>
load_sidebar_artwork_cache(url) -> Option<Vec<u8>>
```

## Performance Benefits

1. **Faster Startup**: Database queries replace filesystem scans
2. **Reduced I/O**: Skip filesystem checks for non-existent entries
3. **Smarter Prefetching**: Know which URLs are placeholders without loading files
4. **Automatic Maintenance**: Old entries cleaned up on startup

## Migration Notes

- **No migration needed**: Database created automatically on first use
- **Backwards compatible**: Existing cache files work without database (slower)
- **Incremental population**: Database populated as files are saved/loaded

## File Structure

```
~/.cache/TempRS/
├── cache.db                    # SQLite database (NEW)
├── artwork/
│   ├── a3f2e1b8...c7d9.jpg    # Main player artwork
│   └── ...
├── sidebar_artwork/
│   ├── b4e3a2c1...d8e0.jpg    # Sidebar thumbnails
│   └── ...
└── audio/
    └── ...
```

## Database Operations

### Automatic Operations

These happen automatically during normal app usage:

- **On save**: `set_entry()` called by `save_artwork_cache()`, `save_sidebar_artwork_cache()`
- **On load**: `is_cached()` called by `load_artwork_cache()`, `load_sidebar_artwork_cache()`
- **On startup**: `cleanup_old_entries(7)` removes entries older than 7 days

### Manual Operations (Available for Future Use)

```rust
// Get metadata for specific entry
let entry = db.get_entry(url, "artwork")?;
println!("Cached at: {}, Size: {}", entry.cached_at, entry.file_size);

// Get all artworks
let artworks = db.get_all_by_type("artwork");
println!("Total artworks: {}", artworks.len());

// Remove specific entry
db.remove_entry(url, "artwork")?;

// Clear all artworks
db.clear_cache_type("artwork")?;

// Get total count
let count = db.get_cache_count();
println!("Total cache entries: {}", count);
```

## Error Handling

The integration is designed to fail gracefully:

- **Database errors**: Fall back to filesystem-only mode
- **Missing database**: Created automatically on first access
- **Corrupted database**: Can be deleted; will be recreated

All database operations are wrapped in `if let Ok(db) = CacheDB::new()` to handle failures silently.

## Future Enhancements

Possible improvements enabled by database:

1. **Cache size limits**: Track total size, evict oldest when limit reached
2. **Usage tracking**: Record access times, evict least-recently-used
3. **URL expiry**: Track URL TTL from API headers
4. **Integrity checks**: Verify file_hash matches actual file
5. **Cache UI**: Display statistics in settings screen
6. **Selective cleanup**: Clear only placeholders, or only old files

## Testing

To verify the integration:

1. **Delete cache**: `rm -rf ~/.cache/TempRS/`
2. **Run app**: Loads playlist, fetches artworks
3. **Check database**: `sqlite3 ~/.cache/TempRS/cache.db "SELECT * FROM cache_entries;"`
4. **Restart app**: Artworks should load instantly from cache
5. **Check logs**: No repeated fetch attempts for failed URLs (placeholders cached)

## Troubleshooting

**Cache not loading on restart:**
- Check database exists: `ls -la ~/.cache/TempRS/cache.db`
- Verify entries: `sqlite3 ~/.cache/TempRS/cache.db "SELECT COUNT(*) FROM cache_entries;"`

**Cache growing too large:**
- Reduce cleanup days in `player_app.rs`: `cleanup_old_cache_db(3)` (3 days instead of 7)
- Manual cleanup: `rm -rf ~/.cache/TempRS/artwork/*` (database will self-heal)

**Database corruption:**
- Delete and recreate: `rm ~/.cache/TempRS/cache.db` (will be recreated on next run)
