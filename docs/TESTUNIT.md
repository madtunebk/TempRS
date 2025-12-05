# TempRS API Test Utility

A standalone Rust binary to test SoundCloud API endpoints.

## Build

```bash
cargo build --release --bin testunit
```

The binary will be created at: `target/release/testunit`

## Usage

```bash
./target/release/testunit
```

The program will:
1. **Automatically load your token** from `~/.config/TempRS/tokens.db`
   - Decrypts the token using your machine fingerprint
   - Validates that the token hasn't expired
   - Falls back to manual input if database is empty or decryption fails
2. Test the following endpoints:
   - `/me` - Get user information
   - `/me/playlists` - Get user playlists (limit 10)
   - `/me/activities` - Get user activities (limit 10)
3. Display formatted results for each test

## Getting Your Token

**Method 1: Automatic (Recommended)**
If you've already logged into TempRS, the token will be automatically loaded from the encrypted database. Just run:
```bash
./target/release/testunit
```

**Method 2: Manual Input**
If the database is empty or you want to test with a different token:
1. Run TempRS with logging enabled:
   ```bash
   RUST_LOG=info ./target/release/TempRS
   ```
2. Complete the OAuth login flow
3. Look for the access token in the logs
4. Paste it when prompted by testunit

## Example Output

**With Stored Token:**
```
===========================================
  TempRS SoundCloud API Test Utility
===========================================

✓ Loaded token from database
  User machine fingerprint: a1b2c3d4e5f6g7h8
  Token expires at: 1732800000

===========================================
  Running API Tests
===========================================

[Testing /me endpoint]
Status: 200 OK
✓ Success!
  User ID: 123456789
  Username: your_username
  Followers: 42

...
```

**Without Stored Token:**
```
===========================================
  TempRS SoundCloud API Test Utility
===========================================

✗ Could not load token from database: No token found

Please enter your OAuth access token manually:
> [paste token here]

## Implementation Details

- **Automatic token loading**: Reuses the same `TokenStore` and encryption logic as TempRS
- **Machine-bound security**: Tokens are encrypted with AES-256-GCM using machine fingerprint
- **Cross-machine safety**: Token decryption only works on the same machine where it was created
- Uses `reqwest` with blocking client for synchronous HTTP requests
- Deserializes JSON responses with `serde`
- Sends OAuth token via `Authorization: OAuth <token>` header
- Validates token expiration before making API calls
- Handles errors gracefully with descriptive output
✓ Success!
  User ID: 123456789
  Username: your_username
  Followers: 42

[Testing /me/playlists endpoint]
Status: 200 OK
✓ Success!
  Total playlists: 5

  Playlist #1
    ID: 987654321
    Title: My Favorite Tracks
    Tracks: 120

...

===========================================
  Tests Complete
===========================================
```

## Implementation Details

- Uses `reqwest` with blocking client for synchronous HTTP requests
- Deserializes JSON responses with `serde`
- Sends OAuth token via `Authorization: OAuth <token>` header
- Handles errors gracefully with descriptive output
