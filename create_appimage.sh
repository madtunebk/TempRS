#!/bin/bash
set -e

# TempRS AppImage Build Script

echo "==================================="
echo "Building TempRS AppImage"
echo "==================================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
APP_NAME="TempRS"
APPDIR="create_app/AppDir"
APPIMAGETOOL="create_app/appimagetool-x86_64.AppImage"

# Parse arguments
FORCE_BUILD=false
SKIP_BUILD=false
for arg in "$@"; do
    case $arg in
        --force|-f)
            FORCE_BUILD=true
            shift
            ;;
        --skip-build|-s)
            SKIP_BUILD=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --force, -f       Force rebuild even if sources unchanged"
            echo "  --skip-build, -s  Skip cargo build, use existing binary"
            echo "  --help, -h        Show this help message"
            exit 0
            ;;
    esac
done

# Step 1: Smart build system
if [ "$SKIP_BUILD" = true ]; then
    echo -e "${YELLOW}[1/5]${NC} Skipping cargo build (--skip-build flag)"

    if [ ! -f "target/release/TempRS" ]; then
        echo -e "${RED}Error: Binary not found and --skip-build specified!${NC}"
        echo "Run without --skip-build to build first"
        exit 1
    fi

    echo -e "${GREEN}✓ Using existing binary${NC}"
else
    echo -e "${YELLOW}[1/5]${NC} Checking if rebuild is needed..."

    NEEDS_BUILD=false

    # Check if binary exists
    if [ ! -f "target/release/TempRS" ]; then
        echo -e "${YELLOW}→ Binary not found, build required${NC}"
        NEEDS_BUILD=true
    elif [ "$FORCE_BUILD" = true ]; then
        echo -e "${YELLOW}→ Force build requested (--force flag)${NC}"
        NEEDS_BUILD=true
    else
        # Check if any Rust source files are newer than the binary
        BINARY_TIME=$(stat -c %Y "target/release/TempRS" 2>/dev/null || echo 0)

        # Find newest source file
        NEWEST_SRC=$(find src -type f \( -name "*.rs" -o -name "*.toml" \) -printf '%T@\n' 2>/dev/null | sort -rn | head -1)
        NEWEST_SRC=${NEWEST_SRC:-0}
        NEWEST_SRC_INT=${NEWEST_SRC%.*}  # Remove decimal part

        # Also check Cargo.toml
        CARGO_TOML_TIME=$(stat -c %Y "Cargo.toml" 2>/dev/null || echo 0)

        if [ "$NEWEST_SRC_INT" -gt "$BINARY_TIME" ] || [ "$CARGO_TOML_TIME" -gt "$BINARY_TIME" ]; then
            echo -e "${YELLOW}→ Source files modified, rebuild required${NC}"
            NEEDS_BUILD=true
        else
            echo -e "${GREEN}→ Binary is up to date, skipping rebuild${NC}"
            echo -e "${GREEN}   (use --force to rebuild anyway)${NC}"
            NEEDS_BUILD=false
        fi
    fi

    if [ "$NEEDS_BUILD" = true ]; then
        echo -e "${YELLOW}→ Building release binary...${NC}"
        cargo build --release

        if [ ! -f "target/release/TempRS" ]; then
            echo -e "${RED}Error: Release binary not found after build!${NC}"
            exit 1
        fi

        echo -e "${GREEN}✓ Release binary built successfully${NC}"
    else
        echo -e "${GREEN}✓ Using existing binary (no changes detected)${NC}"
    fi
fi

# Step 2: Copy binary to AppDir
echo -e "${YELLOW}[2/5]${NC} Copying binary to AppDir..."
mkdir -p "$APPDIR/usr/bin"
cp target/release/TempRS "$APPDIR/usr/bin/"
chmod +x "$APPDIR/usr/bin/TempRS"
echo -e "${GREEN}✓ Binary copied${NC}"

# Step 3: Verify icon exists
echo -e "${YELLOW}[3/5]${NC} Checking icon..."
if [ ! -f "$APPDIR/usr/share/icons/hicolor/256x256/apps/TempRS.png" ]; then
    echo -e "${YELLOW}⚠ Icon not found, creating placeholder${NC}"
    mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
    # If you have an icon, copy it here
fi

# Step 4: Verify or create desktop file
echo -e "${YELLOW}[4/5]${NC} Verifying desktop file..."
mkdir -p "$APPDIR/usr/share/applications"
if [ ! -f "$APPDIR/TempRS.desktop" ]; then
    echo -e "${YELLOW}⚠ Desktop file not found; creating default desktop file at $APPDIR/TempRS.desktop${NC}"
    cat > "$APPDIR/TempRS.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=$APP_NAME
Exec=$APP_NAME
Icon=$APP_NAME
Categories=AudioVideo;Player;Music;Utility;
Terminal=false
EOF
fi
cp "$APPDIR/TempRS.desktop" "$APPDIR/usr/share/applications/"
# Create AppRun wrapper to prefer host's loader paths if needed
cat > "$APPDIR/AppRun" <<'APPRUN'
#!/usr/bin/env sh
set -e
HERE="$(dirname "$(readlink -f "$0")")"
BINARY_PATH="$HERE/usr/bin/TempRS"
for loader in \
  /lib64/ld-linux-x86-64.so.2 \
  /lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 \
  /lib/ld-linux-x86-64.so.2; do
  if [ -x "$loader" ]; then
    exec "$loader" --library-path "$HERE/usr/lib:$HERE/usr/lib64:$LD_LIBRARY_PATH" "$BINARY_PATH" "$@"
  fi
done
exec "$BINARY_PATH" "$@"
APPRUN
chmod +x "$APPDIR/AppRun"

echo -e "${GREEN}✓ Desktop file verified and AppRun created${NC}"

# Step 5: Create AppImage
echo -e "${YELLOW}[5/5]${NC} Creating AppImage..."
if [ ! -f "$APPIMAGETOOL" ]; then
    echo -e "${RED}Error: appimagetool not found!${NC}"
    echo "Download it from: https://github.com/AppImage/AppImageKit/releases"
    exit 1
fi

chmod +x "$APPIMAGETOOL"

# Remove old AppImage if exists
rm -f TempRS-x86_64.AppImage

# Create the AppImage
( cd create_app && ARCH=x86_64 ./appimagetool-x86_64.AppImage AppDir ../TempRS-x86_64.AppImage )

# Try to find the generated AppImage in likely locations
APPIMAGE_PATH=""
CANDIDATES=("./TempRS-x86_64.AppImage" "create_app/TempRS-x86_64.AppImage" "create_app/../TempRS-x86_64.AppImage")
for p in "${CANDIDATES[@]}"; do
  if [ -f "$p" ]; then
    APPIMAGE_PATH="$p"
    break
  fi
done

# Fallback: search for any TempRS*.AppImage in repo (maxdepth 2)
if [ -z "$APPIMAGE_PATH" ]; then
  found=$(find . -maxdepth 2 -type f -name 'TempRS*.AppImage' -print -quit || true)
  if [ -n "$found" ]; then
    APPIMAGE_PATH="$found"
  fi
fi

if [ -n "$APPIMAGE_PATH" ]; then
  # Move to repo root if needed
  if [ "$(realpath "$APPIMAGE_PATH")" != "$(realpath ./TempRS-x86_64.AppImage)" ]; then
    mv "$APPIMAGE_PATH" ./TempRS-x86_64.AppImage
    APPIMAGE_PATH="./TempRS-x86_64.AppImage"
  fi
  chmod +x "$APPIMAGE_PATH"
  echo -e "${GREEN}✓ AppImage created successfully!${NC}"
  echo ""
  echo "==================================="
  echo -e "${GREEN}✓ Build complete!${NC}"
  echo "==================================="
  echo ""
  echo "AppImage location: $(pwd)/TempRS-x86_64.AppImage"
  echo "Size: $(du -h TempRS-x86_64.AppImage | cut -f1)"
  echo ""
  echo "To run: ./TempRS-x86_64.AppImage"
else
  echo -e "${RED}Error: AppImage creation failed!${NC}"
  echo "appimagetool may have printed success but no .AppImage was found in expected locations."
  echo "Check create_app/ for output or run appimagetool manually to inspect its output."
  exit 1
fi
