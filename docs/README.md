# .TMRS - Shared Shader Files

This folder contains shader files shared between the shader editor and TempRS.

## Structure

```
.TMRS/
├── shaders/           # Exported .wgsls shader files
└── README.md          # This file
```

## Shader Format

Shaders exported from the editor use the **WGSL Multi-Pass** format:

- **Boilerplate** - Uniforms and VSOut struct
- **Texture Bindings** - Multi-pass texture samplers
- **Vertex Shader** - `vs_main` entry point
- **Fragment Shaders** - Unique names per buffer:
  - `fs_main_image` - MainImage (compositor)
  - `fs_buffer_a` - Buffer A
  - `fs_buffer_b` - Buffer B
  - `fs_buffer_c` - Buffer C
  - `fs_buffer_d` - Buffer D

## Pipeline Alignment

Both the editor and TempRS use the same multi-pass pipeline architecture:

1. Render Buffer A-D to offscreen textures
2. Render MainImage using textures from buffers
3. Output final image to screen

## Usage

**Export from editor:**
- Edit shaders in the editor
- Click "Export" button
- Save to `.TMRS/shaders/`

**Load in TempRS:**
- TempRS loads `.wgsls` files from this folder
- Automatically detects multi-pass shaders
- Renders with full audio reactivity
