# Setup Guide - Shader Editor + TempRS Integration

## Overview

TempRS supports audio-reactive WGSL shaders with multi-pass rendering. You can create shaders using the [wgsls_editor](https://github.com/madtunebk/wgsls_editor) and have them hot-reload in TempRS automatically.

## Step 1: Install wgsls_editor

```bash
# Clone shader editor
git clone https://github.com/madtunebk/wgsls_editor.git
cd wgsls_editor/
cargo build --release
```

## Step 2: Shader Workflow

### Creating Shaders

1. **Edit in wgsls_editor:**
   ```bash
   cd wgsls_editor/
   cargo run --release
   ```
   - Create multi-pass shader using Buffer A-D tabs
   - Test with audio reactivity in the editor preview
   - MainImage tab combines all buffers

2. **Export to JSON:**
   - Click "Export" button in the editor
   - Save as JSON format
   - Copy exported JSON to: `~/.cache/TempRS/shaders/shader.json`

3. **Auto-reload in TempRS:**
   - TempRS checks for changes every 2 seconds
   - Validates shader with naga before loading
   - Falls back to previous shader if validation fails
   - No restart needed - changes appear automatically!

### File Locations

**TempRS reads from:**
- `~/.cache/TempRS/shaders/shader.json` - Your custom shader (hot-reloadable)
- Fallback: `src/assets/shards/demo_multipass.json` (embedded, if cache missing)

**Editor exports to:**
- Any location you choose when clicking "Export"
- Manually copy to TempRS cache folder for hot-reload

### Shader Format

The JSON format supports both single-pass and multi-pass shaders:

**Single-pass** (minimal):
```json
{
  "version": "1.0",
  "fragment": "@fragment\nfn fs_main(in: VSOut) -> @location(0) vec4<f32> { ... }"
}
```

**Multi-pass** (full):
```json
{
  "version": "1.0",
  "encoding": "base64",
  "fragment": "base64_encoded_mainimage_shader",
  "buffer_a": "base64_encoded_buffer_a_shader",
  "buffer_b": "base64_encoded_buffer_b_shader",
  "buffer_c": "base64_encoded_buffer_c_shader",
  "buffer_d": "base64_encoded_buffer_d_shader"
}
```

**Auto-injection:**
TempRS automatically injects:
- ✅ Uniforms struct (time, audio_bass, audio_mid, audio_high, resolution)
- ✅ VSOut struct (position, uv)
- ✅ Vertex shader (vs_main) - if not provided
- ✅ Texture bindings (group 1) - only for MainImage if buffers exist

You only write the fragment shader logic!

## Step 3: Development Cycle

**Rapid iteration workflow:**

1. Edit shader in wgsls_editor → test in editor preview
2. Export to JSON → copy to `~/.cache/TempRS/shaders/shader.json`
3. TempRS auto-reloads within 2 seconds
4. See changes in Now Playing view immediately
5. Repeat!

**No need to:**
- Restart TempRS
- Recompile anything
- Manually inject boilerplate

## Tips

### Debugging

**Enable shader logging:**
```bash
RUST_LOG=debug cargo run --release --bin TempRS
```

**Watch for validation errors:**
```
[Shader] Hot-reload failed: validation error at line 42
[Shader] Keeping previous shader - fix errors and save again
```

### Testing

1. Start with demo shader: `src/assets/shards/demo_multipass.json`
2. Export from editor and compare outputs
3. Audio reactivity should match between editor and TempRS

### Common Issues

**"Shader validation failed"**
- Check naga error message in logs
- Verify uniforms struct matches expected structure
- Ensure fragment function returns `vec4<f32>`

**"File not found"**
- Create cache directory: `mkdir -p ~/.cache/TempRS/shaders/`
- Check file permissions
- Verify JSON is valid (not corrupted)

**"Shader not reloading"**
- Wait at least 2 seconds after saving
- Check file modification time changed
- Look for validation errors in logs
- Buffer shader missing or invalid
- Check TempRS logs for validation errors
- Gracefully skips missing buffers
