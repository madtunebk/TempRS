# TODO: Enable Embedded Image Loading in TempRS

## Status
The infrastructure for loading embedded images from shader JSON is **complete** but not yet **wired up**.

## What's Done ✅
1. `shader_constants.rs` - iChannel0-3 declarations added to SHADER_BOILERPLATE
2. `multi_buffer_pipeline.rs` - All bindings (8-15) added, `new_with_images()` function created
3. `shader_json.rs` - `ichannel0-3` fields added, `decode_embedded_images()` helper function

## What's Missing ⚠️
The actual shader loading code in TempRS needs to be updated to use `new_with_images()` instead of `new()`.

## How to Fix

### Step 1: Find where shaders are loaded
Search for where `MultiPassPipelines::new()` is called in your TempRS codebase:
```bash
grep -r "MultiPassPipelines::new" src/
```

### Step 2: Update the shader loading code
Replace the old pipeline creation:

```rust
// OLD CODE (probably in src/app/player_app.rs or similar):
let sources = shader_json.to_shader_map();
let pipeline = MultiPassPipelines::new(
    device, 
    format, 
    screen_size, 
    &sources
)?;
```

With the new version that loads embedded images:

```rust
// NEW CODE:
let sources = shader_json.to_shader_map();
let embedded_images = shader_json.decode_embedded_images();
let pipeline = MultiPassPipelines::new_with_images(
    device, 
    format, 
    screen_size, 
    &sources, 
    &embedded_images
)?;
```

### Step 3: Test
1. Export a shader with embedded images from the editor
2. Load it in TempRS
3. The images should now appear in iChannel0-3

## Example Test Shader
```wgsl
@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let color = textureSample(iChannel0, iChannel0Sampler, in.uv);
    return color;
}
```

This shader should display the embedded image if everything is wired correctly.

## Debug Logging
The `decode_embedded_images()` function logs:
```
[INFO] Decoded embedded image 0 (12345 bytes)
```

If you see this log but no image appears, the problem is in the GPU texture upload (check `multi_buffer_pipeline.rs` line ~730).

## Notes
- Images are decoded from base64 and loaded as WGPU textures
- If no images are embedded, dummy textures (black) are used
- NO automatic artwork filling - only uses what's explicitly in the JSON
