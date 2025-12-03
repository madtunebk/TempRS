# GitHub Publishing Checklist

Before pushing to GitHub, replace your real credentials with dummy ones:

## Step 1: Create GitHub Version of main.rs

Replace lines 19-23 in `src/main.rs` with:

```rust
// SoundCloud OAuth Credentials
// ⚠️ THESE ARE DUMMY/PLACEHOLDER VALUES - Replace with your own credentials
// See CREDENTIALS_SETUP.md for instructions on getting working credentials
pub const SOUNDCLOUD_CLIENT_ID: &str = "YOUR_SOUNDCLOUD_CLIENT_ID_HERE";
pub const SOUNDCLOUD_CLIENT_SECRET: &str = "YOUR_SOUNDCLOUD_CLIENT_SECRET_HERE";
```

## Step 2: Verify .gitignore

Ensure `.gitignore` contains:
```
# Sensitive files
tokens.db
token_data.json
oauth_token*
*.env
```

## Step 3: Test Build (Should Fail)

With dummy credentials, build should compile but app will fail to authenticate:
```bash
cargo build --release
```

## Step 4: Commit and Push

```bash
git add .
git commit -m "Prepare for public release - use dummy credentials"
git push origin main
```

## Step 5: Update Your Local Version

After pushing, restore your real credentials locally:

```bash
# Keep your working credentials in a separate file (not tracked)
# Or restore them manually in src/main.rs
```

## Security Reminders

✅ Real credentials stay local only
✅ GitHub gets dummy credentials
✅ Users must provide their own credentials
✅ OAuth tokens encrypted in ~/.config/TempRS/tokens.db (excluded from git)

## Alternative: Use Git Assume Unchanged

Keep real credentials locally but exclude from commits:

```bash
# After committing dummy version
git update-index --assume-unchanged src/main.rs

# Then restore your real credentials locally
# Git will ignore changes to this file
```

To start tracking again:
```bash
git update-index --no-assume-unchanged src/main.rs
```
