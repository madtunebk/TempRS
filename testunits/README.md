# Test Units

Standalone test binaries for API endpoints and features. These are kept separate from the main app to avoid bloating the binary.

## Available Tests

### `play_history_test.rs`
Tests the SoundCloud Play History API endpoint.

**Endpoint:** `https://api-v2.soundcloud.com/me/play-history`

**Usage:**
```bash
# Set your OAuth token (get it from the main app after login)
export SOUNDCLOUD_TOKEN='your_access_token_here'

# Run the test
cargo run --release --bin play_history_test

# Or use the helper script
./test_play_history.sh
```

**What it tests:**
- API authentication with Bearer token
- Response structure parsing
- Play history collection format
- Track metadata (title, artist, duration)
- Timestamps (played_at)

## Adding New Tests

1. Create new test file in `testunits/` folder
2. Add binary entry to `Cargo.toml`:
   ```toml
   [[bin]]
   name = "your_test_name"
   path = "testunits/your_test_name.rs"
   ```
3. Build and run:
   ```bash
   cargo run --release --bin your_test_name
   ```

## Notes

- Test binaries are NOT included in the main TempRS app
- Each test is a standalone executable
- Use for API exploration, debugging, and validation
- Keep test code simple and focused on one endpoint/feature
