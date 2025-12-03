# API Retry System

## Overview

TempRS now includes comprehensive retry protection for all SoundCloud API calls, providing resilience against transient network errors and server-side issues like 504 Gateway Timeout.

## Features

### Automatic Retry Logic
- **Exponential Backoff**: Delays increase with each retry (500ms → 1s → 2s)
- **Max Attempts**: 3 attempts per request (initial + 2 retries)
- **Smart Detection**: Only retries on transient, recoverable errors

### Retryable HTTP Status Codes
- `408` Request Timeout
- `429` Too Many Requests (rate limiting)
- `500` Internal Server Error
- `502` Bad Gateway
- `503` Service Unavailable
- `504` Gateway Timeout

### Configuration
- **Request Timeout**: 30 seconds per attempt
- **Base Delay**: 500ms
- **Max Delay**: 2000ms (after 2nd retry)
- **Connection Pooling**: Enabled via shared HTTP client
- **TCP Keepalive**: Enabled for long-lived connections

## Implementation

### Core Functions (`src/utils/http.rs`)

#### `retry_get_with_auth(url, token)` 
- Standard GET requests with OAuth token
- Used by most API endpoints (tracks, playlists, search, etc.)

#### `retry_post_with_auth(url, token)`
- POST requests with OAuth token  
- Used for like/unlike operations

#### `retry_delete_with_auth(url, token)`
- DELETE requests with OAuth token
- Used for unlike operations

#### `retry_request<F>(closure)` (Generic)
- Flexible wrapper for custom request patterns
- Available for future complex use cases

#### `is_retryable_status(status_code)`
- Determines if error should trigger retry
- Returns `true` for 408, 429, 500, 502, 503, 504

## Updated API Modules

All 20 API call sites now use retry logic:

### `src/api/tracks.rs` (3 calls)
- `fetch_track_by_id()` - Single track retrieval
- `fetch_related_tracks()` - Related tracks discovery
- `load_next_search_page_smart()` - Paginated search results

### `src/api/likes.rs` (7 calls)
- `fetch_user_liked_tracks()` - User's liked tracks
- `fetch_user_playlists()` - Created + liked playlists (2 calls)
- `fetch_user_liked_playlists()` - Liked playlists only
- `fetch_user_tracks()` - User's uploaded tracks
- `like_track()` - Like a track (POST)
- `unlike_track()` - Unlike a track (DELETE)

### `src/api/playlists.rs` (3 calls)
- `fetch_playlist_by_id()` - Full playlist with pagination (2 calls)
- `fetch_playlist_chunks()` - Chunked playlist loading

### `src/api/users.rs` (2 calls)
- `fetch_track_favoriters()` - Users who liked a track
- `fetch_user_likes()` - User's favorite tracks

### `src/api/search.rs` (4 calls)
- `search_tracks_smart()` - Smart paginated search
- `search_tracks()` - Basic track search
- `search_playlists()` - Playlist search
- `search_playlists_paginated()` - Paginated playlist search

### `src/api/activities.rs` (1 call)
- `fetch_recent_activities()` - User's recent listening history

## Logging

All retry attempts are logged with:
```
[HTTP Retry] Status 504 from https://api.soundcloud.com/... Retrying in 500ms... (attempt 1/3)
[HTTP Retry] Status 503 from https://api.soundcloud.com/... Retrying in 1000ms... (attempt 2/3)
```

Logs include:
- HTTP status code
- Request URL
- Retry delay
- Attempt number

## Migration Notes

### What Changed
- **Removed**: Individual `reqwest::Client::new()` instances per API call
- **Added**: Shared HTTP client via `crate::utils::http::client()`
- **Replaced**: Direct `.send().await?` calls with `retry_*_with_auth()` wrappers
- **Benefit**: Consistent retry behavior, connection pooling, reduced overhead

### Breaking Changes
None - all changes are internal implementation details. Public API signatures unchanged.

## Testing

### Manual Testing
```bash
# Build with retry system
cargo build --release --bin TempRS

# Run and monitor logs for retry behavior
RUST_LOG=debug cargo run --release --bin TempRS
```

### Simulating Failures
To test retry logic with simulated 504 errors:
1. Temporarily modify `is_retryable_status()` to always return `true`
2. Or use network tools to simulate intermittent failures
3. Check logs for retry attempts

## Performance Impact

### Benefits
- **Connection Reuse**: Shared client enables HTTP connection pooling
- **Reduced Failures**: Automatic recovery from transient errors
- **Better UX**: Silent retry instead of user-facing errors

### Overhead
- **Minimal**: Only triggers on actual errors (no overhead for success)
- **Bounded**: Max 3 attempts prevents infinite loops
- **Async**: Non-blocking delays (tokio::time::sleep)

## Future Enhancements

Potential improvements:
- [ ] Configurable retry limits (env var or settings)
- [ ] Adaptive backoff based on `Retry-After` headers
- [ ] Circuit breaker pattern for prolonged outages
- [ ] Metrics/telemetry for retry rates
- [ ] Jitter to prevent thundering herd

## Related Files

- `src/utils/http.rs` - Retry logic implementation
- `src/api/*.rs` - All API modules using retry system
- `.github/copilot-instructions.md` - Project architecture docs

## Build Status

✅ Clean build with 0 warnings  
✅ All 20 API call sites migrated  
✅ Backwards compatible with existing code  
