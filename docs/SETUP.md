# Setup Guide - Editor + TempRS Integration

## Directory Structure

```
egui_two_windows/           # Shader editor (this repo)
├── .TMRS/                  # Shared shader files (gitignored)
│   ├── shaders/            # Exported .wgsls files
│   ├── README.md           # Usage guide
│   ├── PIPELINE_SPEC.md    # Technical pipeline spec
│   └── SETUP.md            # This file
├── TempRS/                 # TempRS repo (gitignored, clone separately)
└── src/                    # Editor source code
```

## Step 1: Clone TempRS

```bash
cd egui_two_windows/
git clone https://github.com/madtunebk/TempRS.git
```

Both repos remain separate with independent git histories.

## Step 2: Align TempRS Pipeline

Copy the multi-pass pipeline code from the editor to TempRS:

**Files to reference:**
- `src/utils/multi_buffer_pipeline.rs` - Full pipeline implementation
- `src/utils/shader_constants.rs` - Shared constants

**What TempRS needs:**
1. Parse shader for multiple entry points (`fs_main_image`, `fs_buffer_a`, etc.)
2. Create 5 pipelines (one per entry point)
3. Create 4 offscreen textures (Buffer A-D outputs)
4. Create texture bind group (group 1, bindings 0-7)
5. Render in order: A → B → C → D → MainImage

See `PIPELINE_SPEC.md` for detailed architecture.

## Step 3: Workflow

### Creating Shaders

1. **Edit in shader editor:**
   - Run: `cargo run` (in egui_two_windows/)
   - Create multi-pass shader using Buffer A-D tabs
   - Test with audio reactivity

2. **Export:**
   - Click "Export" button
   - Saves to `.TMRS/shaders/` by default
   - File includes all boilerplate and bindings

3. **Run in TempRS:**
   - TempRS loads from `.TMRS/shaders/`
   - Renders with same multi-pass pipeline
   - Full audio integration

### File Format

Exported `.wgsls` files contain:
- ✅ Boilerplate (Uniforms, VSOut structs)
- ✅ Texture bindings (for multi-pass)
- ✅ Vertex shader (vs_main)
- ✅ All fragment shaders (fs_main_image, fs_buffer_a, etc.)

**Self-contained** - No manual editing needed!

## Step 4: Git Ignore

Both `.TMRS/` and `TempRS/` are already in `.gitignore`:

```gitignore
# TempRS integration
TempRS/                  # TempRS repo (separate git)
.TMRS/                   # Shared shader files
```

This keeps:
- ✅ Editor repo clean (no TempRS commits)
- ✅ TempRS repo clean (no editor commits)
- ✅ Shader files local (not version controlled)

## Tips

### Debugging Exports

Run with debug logging to see export structure:
```bash
RUST_LOG=debug cargo run
```

When you export, you'll see:
```
=== EXPORT DEBUG ===
Export length: 5432 bytes
Export preview (first 500 chars):
// Exported: 2025-12-05 19:50:00
...
===================
```

### Testing Pipeline Alignment

1. Export demo shader from editor
2. Load in TempRS
3. Should render identical output
4. Audio reactivity should match

### Common Issues

**"unknown identifier: buffer_a_texture"**
- TempRS needs texture bindings (group 1)
- See `PIPELINE_SPEC.md` for bind group layout

**"redefinition of fs_main"**
- Old export format (fixed)
- Re-export shader with unique names

**Empty/black buffers**
- Buffer shader missing or invalid
- Check TempRS logs for validation errors
- Gracefully skips missing buffers
