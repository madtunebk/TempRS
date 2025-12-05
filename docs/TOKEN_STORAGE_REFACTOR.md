# Token Storage Refactor

## Overview
Restructured token storage from single encrypted JSON blob to **separate encrypted database columns** for better token management and easier refresh logic.

## Database Schema Changes

### Old Schema (JSON blob)
```sql
CREATE TABLE tokens (
    id INTEGER PRIMARY KEY,
    token_data TEXT NOT NULL,  -- Single encrypted JSON string
    created_at INTEGER NOT NULL
)
```

### New Schema (Separate columns)
```sql
CREATE TABLE tokens (
    id INTEGER PRIMARY KEY,
    access_token TEXT NOT NULL,      -- Encrypted
    refresh_token TEXT,               -- Encrypted (nullable)
    token_type TEXT NOT NULL,         -- Plain text (e.g., "Bearer")
    expires_at INTEGER NOT NULL,      -- Unix timestamp
    created_at INTEGER NOT NULL,      -- Unix timestamp
    machine_fp TEXT NOT NULL          -- Encrypted machine fingerprint
)
```

## Benefits

### 1. **Easier Token Validation**
- Can directly query `expires_at` without decrypting entire token
- Fast expiry checks without full deserialization

### 2. **Better Refresh Logic**
- Can check if `refresh_token` exists without loading access token
- New method: `get_token_for_refresh()` loads expired tokens for refresh attempts

### 3. **Improved Debugging**
- Separate fields make logging clearer
- Can see token metadata without decryption
- Better error messages for specific field failures

### 4. **Security Maintained**
- Access token, refresh token, and machine fingerprint remain AES-256-GCM encrypted
- Same machine-bound encryption key derivation
- Token type stored as plain text (not sensitive)

## Code Changes

### `src/utils/token_store.rs`
1. **Updated Schema**: Separate columns instead of JSON blob
2. **save_token()**: Encrypts each field separately
3. **load_token()**: Decrypts fields individually with better error handling
4. **New method**: `get_token_for_refresh()` - loads token even if expired
5. **Enhanced logging**: Detailed token validity checks

### `src/utils/oauth.rs`
1. **New method**: `get_token_for_refresh()` - exposes token store method
2. **Better logging**: Clear status messages for token checks

### `src/app/splash.rs`
1. **Cleaner refresh logic**: Uses `get_token_for_refresh()` for expired tokens
2. **Better status messages**: Emoji indicators (✅, 🔄, ❌, ⚠️, ℹ️) for clarity
3. **Simplified flow**: Clearer separation between refresh and login paths

## Token Flow

### On App Start
1. **Check valid token** → Has unexpired token? → Proceed to main screen
2. **Check refresh** → Token expired but has refresh_token? → Auto-refresh
3. **Show login** → No token or refresh failed? → Show login button

### Refresh Process
```
Token expired → get_token_for_refresh() → Extract refresh_token → 
API call → Save new token → Next render detects valid token → Main screen
```

### Security Notes
- All sensitive fields (access_token, refresh_token, machine_fp) remain encrypted
- Machine fingerprint validation prevents cross-machine token use
- Encryption key derived from machine fingerprint + salt
- Each encrypted field has its own random nonce (AES-GCM)

## Migration
- Old tokens will fail to load (different schema)
- Users will need to re-login once
- No data loss - just requires re-authentication
- Token cache automatically cleaned on first run

## Testing
Run with verbose logging to see token lifecycle:
```bash
RUST_LOG=debug cargo run --release --bin TempRS
```

Look for:
- `[TokenStore]` - Token storage operations
- `[OAuth]` - OAuth manager operations  
- `[Splash]` - Authentication flow status
