#!/bin/bash

# Test SoundCloud Play History API
# Usage: ./test_play_history.sh

echo "=== SoundCloud Play History API Test ==="
echo ""

# Token is now loaded from database automatically
# No need to set SOUNDCLOUD_TOKEN environment variable
echo "Token will be loaded from ~/.config/TempRS/tokens.db"
echo ""

echo "Building play_history_test..."
cargo build --release --bin play_history_test

if [ $? -eq 0 ]; then
    echo ""
    echo "Running test..."
    echo ""
    RUST_LOG=info ./target/release/play_history_test
else
    echo "Build failed!"
    exit 1
fi
