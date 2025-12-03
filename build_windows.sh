#!/bin/bash
set -e

# Build type: debug (default) or release
BUILD_TYPE=${1:-debug}
TARGET="x86_64-pc-windows-msvc"  # Windows 64-bit MSVC target
FLAGS="--bin TempRS --target $TARGET"

if [ "$BUILD_TYPE" == "release" ]; then
    echo "Building release Windows binary..."
    FLAGS="$FLAGS --release"
else
    echo "Building debug Windows binary..."
fi

echo "Building with: cargo xwin build $FLAGS"
cargo xwin build $FLAGS

echo ""
echo "Build complete!"
if [ "$BUILD_TYPE" == "release" ]; then
    echo "Binary should be located at: target/$TARGET/release/TempRS.exe"
else
    echo "Binary should be located at: target/$TARGET/debug/TempRS.exe"
fi

# Optional: Collect DLLs and create a distribution zip
if [ "$BUILD_TYPE" == "release" ] && [ "$2" == "package" ]; then
    echo "Creating distribution package..."
    DIST_DIR="dist/windows"
    mkdir -p "$DIST_DIR"
    
    # Copy executable
    cp "target/$TARGET/release/TempRS.exe" "$DIST_DIR/"
    
    # TODO: Copy required DLLs if needed
    
    echo "Package created in $DIST_DIR/"
fi
