# RGBA Multi-Buffer Demo

This demo showcases all 4 shader buffers working together to create a color-swapping effect.

## Files

- `rgba_vertex.vert` - Shared vertex shader (load this for all buffers)
- `rgba_buffer_a.frag` - Red channel: horizontal waves
- `rgba_buffer_b.frag` - Green channel: vertical waves  
- `rgba_buffer_c.frag` - Blue channel: circular spiral
- `rgba_buffer_d.frag` - Alpha/intensity: animated grid
- `rgba_main.frag` - MainImage: combines all channels with color swapping

## How to Use

1. Load `rgba_vertex.vert` as the vertex shader for all buffers
2. For each buffer tab (MainImage, A, B, C, D), load the corresponding fragment shader:
   - **MainImage**: `rgba_main.frag`
   - **Buffer A**: `rgba_buffer_a.frag`
   - **Buffer B**: `rgba_buffer_b.frag`
   - **Buffer C**: `rgba_buffer_c.frag`
   - **Buffer D**: `rgba_buffer_d.frag`
3. Click "Apply Shader" after loading each one
4. The MainImage will combine all 4 buffers with a time-based color swapping effect

## What Each Buffer Does

- **Buffer A (Red)**: Creates horizontal waves that pulse with bass audio
- **Buffer B (Green)**: Creates vertical waves that pulse with mid-range audio
- **Buffer C (Blue)**: Creates a rotating spiral pattern that pulses with high-frequency audio
- **Buffer D (Intensity)**: Creates an animated grid that modulates the overall brightness

## The Effect

MainImage samples all 4 buffers and combines them into RGB channels:
- Red channel ← Buffer A
- Green channel ← Buffer B  
- Blue channel ← Buffer C
- Overall intensity ← Buffer D

Every few seconds, the color channels swap positions, creating different color combinations!
