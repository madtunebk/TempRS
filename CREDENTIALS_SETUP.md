# SoundCloud API Credentials Setup

TempRS requires SoundCloud API credentials to function. You have two options:

## Option 1: Use Your Own Credentials (Recommended)

1. **Register your app** at SoundCloud Developers (if available)
   - Note: As of 2024-2025, SoundCloud has limited API registration
   - You may need to use existing credentials or alternative methods

2. **Update credentials** in `src/main.rs`:
   ```rust
   pub const SOUNDCLOUD_CLIENT_ID: &str = "YOUR_CLIENT_ID_HERE";
   pub const SOUNDCLOUD_CLIENT_SECRET: &str = "YOUR_CLIENT_SECRET_HERE";
   ```

3. **Build the project**:
   ```bash
   cargo build --release --bin TempRS
   ```

## Option 2: Use Community Credentials

Some community members may share working credentials. Replace the dummy values in `src/main.rs` with working ones.

⚠️ **Warning**: Shared credentials may hit rate limits or be revoked at any time.

## Security Note

- Credentials are stored in plain text in source code for simplicity
- OAuth tokens are encrypted using AES-256-GCM in `~/.config/TempRS/tokens.db`
- Never commit real credentials to public repositories
- The repository ships with dummy/placeholder credentials that won't work

## Current Status

The GitHub repository contains **dummy credentials** that will not work. You must replace them with valid SoundCloud API credentials before building.
