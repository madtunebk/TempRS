# Shader Collection

This directory contains WGSL shader presets for the WebShard Editor.

## Available Shaders

### default.frag / default.vert
**Audio Visualizer Bars**
- Classic frequency spectrum visualizer
- Three-band visualization (bass, mid, high)
- LED-style grid effect
- Great for testing audio input

### psychedelic.frag
**Psychedelic Spiral**
- Fractal noise patterns with FBM (Fractal Brownian Motion)
- Audio-reactive color palette cycling
- Rotating spiral distortion
- Multiple layered effects
- Chromatic aberration and bloom
- **Intensity:** 🔥🔥🔥🔥🔥

### tunnel.frag
**Infinite Tunnel**
- 3D tunnel effect with perspective
- Spiraling grid patterns
- Audio-reactive depth and twist
- Multi-layered with HSV coloring
- Great with bass-heavy music
- **Intensity:** 🔥🔥🔥

### raymarch.frag
**3D Raymarched Boxes**
- Real-time raymarching
- Infinite box grid in 3D space
- Audio-reactive rotation and sizing
- Volumetric glow effects
- Each frequency affects different axis
- **Intensity:** 🔥🔥🔥🔥

### fractal.frag
**Julia Set Fractal**
- Animated Julia set (complex dynamics)
- Audio-reactive parameters
- Orbit trap coloring
- Smooth iteration gradients
- Psychedelic color mapping
- **Intensity:** 🔥🔥🔥🔥

## Shader Uniforms

All shaders have access to these uniforms:

```wgsl
struct Uniforms {
    time: f32,              // Elapsed time in seconds
    audio_bass: f32,        // Low frequency energy (0.0 - 1.0)
    audio_mid: f32,         // Mid frequency energy (0.0 - 1.0)
    audio_high: f32,        // High frequency energy (0.0 - 1.0)
    resolution: vec2<f32>,  // Screen resolution (width, height)
    _pad0: vec2<f32>,       // Padding for alignment
}
```

## Creating Custom Shaders

1. Create a `.frag` file in this directory
2. Use `default.vert` or create a custom vertex shader
3. Include the Uniforms struct and VSOut struct
4. Implement `fs_main` function
5. Load in the editor and enjoy!

### Tips

- Use `uniforms.audio_bass` for heavy, slow effects
- Use `uniforms.audio_high` for fast, detailed effects  
- Use `uniforms.audio_mid` for medium-range modulation
- Combine all three for complex interactions
- Use `uniforms.time` for continuous animation
- Keep aspect ratio in mind: `uv.x *= resolution.x / resolution.y`

## Performance Notes

- **psychedelic.frag**: Heavy (6 FBM iterations + multiple layers)
- **tunnel.frag**: Medium (5 layers, simple math)
- **raymarch.frag**: Heavy (80 raymarching steps)
- **fractal.frag**: Heavy (100 iterations max)
- **default.frag**: Light (simple grid calculation)

For best performance, use lower iteration counts or optimize loops.

---

**Have fun creating insane audio-reactive shaders! 🎨🎵**
