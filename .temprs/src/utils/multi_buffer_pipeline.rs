#![allow(dead_code)]
use std::sync::Arc;
use std::time::Instant;

use crate::utils::{validate_shader, ShaderError};
use eframe::epaint;
use eframe::wgpu::{
    AddressMode, BindGroup, BindGroupLayout, Buffer, CommandEncoder, Device, Extent3d,
    FilterMode, Queue, RenderPipeline, Sampler, SamplerDescriptor, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

// Re-export ShaderUniforms from pipeline module
pub use crate::utils::pipeline::ShaderUniforms;

/// Buffer types for multi-pass rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferKind {
    MainImage,
    BufferA,
    BufferB,
    BufferC,
    BufferD,
}

impl BufferKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BufferKind::MainImage => "MainImage",
            BufferKind::BufferA => "BufferA",
            BufferKind::BufferB => "BufferB",
            BufferKind::BufferC => "BufferC",
            BufferKind::BufferD => "BufferD",
        }
    }
}

/// Single render pass that renders into an offscreen texture
pub struct BufferPass {
    pub kind: BufferKind,
    pub pipeline: RenderPipeline,
    pub target_texture: Texture,
    pub target_view: TextureView,
}

impl BufferPass {
    fn render(&self, encoder: &mut CommandEncoder, uniform_bind_group: &BindGroup) {
        let mut rpass = encoder.begin_render_pass(&eframe::wgpu::RenderPassDescriptor {
            label: Some(&format!("{}_pass", self.kind.as_str())),
            color_attachments: &[Some(eframe::wgpu::RenderPassColorAttachment {
                view: &self.target_view,
                resolve_target: None,
                ops: eframe::wgpu::Operations {
                    load: eframe::wgpu::LoadOp::Clear(eframe::wgpu::Color::BLACK),
                    store: eframe::wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, uniform_bind_group, &[]);
        rpass.draw(0..6, 0..1);
    }
}

/// Helper: create an offscreen texture for a buffer
fn create_color_target(
    device: &Device,
    size: [u32; 2],
    format: TextureFormat,
    label: &str,
) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

/// Multi-pass shader pipeline manager
pub struct MultiPassPipelines {
    pub uniform_buffer: Buffer,
    pub uniform_bind_group_layout: BindGroupLayout,
    pub texture_bind_group_layout: BindGroupLayout,
    pub uniform_bind_group: BindGroup,

    // Optional buffer passes (only created if shader code provided)
    pub buffer_a: Option<BufferPass>,
    pub buffer_b: Option<BufferPass>,
    pub buffer_c: Option<BufferPass>,
    pub buffer_d: Option<BufferPass>,

    // Main image pipeline (always present)
    pub main_image_pipeline: RenderPipeline,
    pub main_texture_bind_group: BindGroup,

    pub sampler: Sampler,
    pub start_time: Instant,
    
    // User-loaded image textures (iChannel0-3 in ShaderToy terms)
    pub user_image_textures: [Option<Texture>; 4],
    pub user_image_views: [Option<TextureView>; 4],
}

impl MultiPassPipelines {
    pub fn new(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        screen_size: [u32; 2],
        sources: &std::collections::HashMap<BufferKind, String>,
        image_paths: &[Option<String>; 4], // Array of 4 image paths for iChannel0-3
    ) -> Result<Self, ShaderError> {
        log::info!(
            "Creating multi-pass shader pipeline (resolution: {}x{})",
            screen_size[0],
            screen_size[1]
        );

        // ===== Uniform buffer =====
        let uniform_size = std::mem::size_of::<ShaderUniforms>() as u64;
        let uniform_buffer = device.create_buffer(&eframe::wgpu::BufferDescriptor {
            label: Some("shader_uniforms"),
            size: uniform_size,
            usage: eframe::wgpu::BufferUsages::COPY_DST | eframe::wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        // ===== Bind group layout: uniforms @group(0) =====
        let uniform_bgl = device.create_bind_group_layout(&eframe::wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform_bgl"),
            entries: &[eframe::wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: eframe::wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: eframe::wgpu::BindingType::Buffer {
                    ty: eframe::wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bg = device.create_bind_group(&eframe::wgpu::BindGroupDescriptor {
            label: Some("uniform_bg"),
            layout: &uniform_bgl,
            entries: &[eframe::wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // ===== Bind group layout: textures @group(1) =====
        let texture_bgl = device.create_bind_group_layout(&eframe::wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bgl"),
            entries: &[
                // Buffer A texture @binding(0)
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Texture {
                        sample_type: eframe::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: eframe::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler A @binding(1)
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Sampler(
                        eframe::wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
                // Buffer B texture @binding(2)
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Texture {
                        sample_type: eframe::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: eframe::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler B @binding(3)
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Sampler(
                        eframe::wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
                // Buffer C texture @binding(4)
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Texture {
                        sample_type: eframe::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: eframe::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler C @binding(5)
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Sampler(
                        eframe::wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
                // Buffer D texture @binding(6)
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Texture {
                        sample_type: eframe::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: eframe::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler D @binding(7)
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Sampler(
                        eframe::wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
                // User image textures (iChannel0-3) @binding(8-15)
                // iChannel0 texture
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Texture {
                        sample_type: eframe::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: eframe::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // iChannel0 sampler
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Sampler(
                        eframe::wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
                // iChannel1 texture
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 10,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Texture {
                        sample_type: eframe::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: eframe::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // iChannel1 sampler
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 11,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Sampler(
                        eframe::wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
                // iChannel2 texture
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 12,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Texture {
                        sample_type: eframe::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: eframe::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // iChannel2 sampler
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 13,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Sampler(
                        eframe::wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
                // iChannel3 texture
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 14,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Texture {
                        sample_type: eframe::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: eframe::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // iChannel3 sampler
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 15,
                    visibility: eframe::wgpu::ShaderStages::FRAGMENT,
                    ty: eframe::wgpu::BindingType::Sampler(
                        eframe::wgpu::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
            ],
        });

        // ===== Shared sampler =====
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("buffer_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        // ===== Load user image textures if provided (iChannel0-3) =====
        let mut user_image_textures: [Option<Texture>; 4] = [None, None, None, None];
        let mut user_image_views: [Option<TextureView>; 4] = [None, None, None, None];
        
        for (i, path_opt) in image_paths.iter().enumerate() {
            if let Some(path) = path_opt {
                match crate::utils::image_loader::load_image_texture(device, queue, path) {
                    Ok((tex, view, dimensions)) => {
                        log::info!("iChannel{} texture loaded: {}x{}", i, dimensions[0], dimensions[1]);
                        user_image_textures[i] = Some(tex);
                        user_image_views[i] = Some(view);
                    }
                    Err(e) => {
                        log::warn!("Failed to load iChannel{} texture: {}", i, e);
                    }
                }
            }
        }

        // ===== BUFFER A: offscreen pass (optional) =====
        let buffer_a = if let Some(buffer_a_src) = sources.get(&BufferKind::BufferA) {
            // Skip if empty or only whitespace
            if buffer_a_src.trim().is_empty() {
                log::debug!("BufferA is empty, skipping");
                None
            } else {
                // Only create if it has actual shader code (not just comments)
                let has_code = buffer_a_src.contains("fn fs_main") || buffer_a_src.contains("@fragment");
                if !has_code {
                    log::debug!("BufferA has no fragment shader code, skipping");
                    None
                } else {
                    log::debug!("Creating BufferA pass");
                    
                    // Try to validate, but skip if it fails (allow partial shaders during development)
                    if let Err(e) = validate_shader(buffer_a_src) {
                        log::warn!("[BufferA] Validation failed, skipping: {}", e);
                        None
                    } else {
                        let buffer_a_module = device.create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
                            label: Some("buffer_a_shader"),
                            source: eframe::wgpu::ShaderSource::Wgsl(buffer_a_src.clone().into()),
                        });

                        let (buffer_a_tex, buffer_a_view) =
                            create_color_target(device, screen_size, format, "buffer_a_target");

                        let buffer_a_pipeline_layout =
                            device.create_pipeline_layout(&eframe::wgpu::PipelineLayoutDescriptor {
                                label: Some("buffer_a_pipeline_layout"),
                                bind_group_layouts: &[&uniform_bgl],
                                push_constant_ranges: &[],
                            });

                        let buffer_a_pipeline =
                            device.create_render_pipeline(&eframe::wgpu::RenderPipelineDescriptor {
                                label: Some("buffer_a_pipeline"),
                                layout: Some(&buffer_a_pipeline_layout),
                                vertex: eframe::wgpu::VertexState {
                                    module: &buffer_a_module,
                                    entry_point: Some("vs_main"),
                                    compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
                                    buffers: &[],
                                },
                                fragment: Some(eframe::wgpu::FragmentState {
                                    module: &buffer_a_module,
                                    entry_point: Some("fs_main"),
                                    compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
                                    targets: &[Some(eframe::wgpu::ColorTargetState {
                                        format,
                                        blend: Some(eframe::wgpu::BlendState::ALPHA_BLENDING),
                                        write_mask: eframe::wgpu::ColorWrites::ALL,
                                    })],
                                }),
                                primitive: eframe::wgpu::PrimitiveState::default(),
                                depth_stencil: None,
                                multisample: eframe::wgpu::MultisampleState::default(),
                                multiview: None,
                                cache: None,
                            });

                        Some(BufferPass {
                            kind: BufferKind::BufferA,
                            pipeline: buffer_a_pipeline,
                            target_texture: buffer_a_tex,
                            target_view: buffer_a_view,
                        })
                    }
                }
            }
        } else {
            None
        };

        // ===== BUFFER B: offscreen pass (optional) =====
        let buffer_b = if let Some(buffer_b_src) = sources.get(&BufferKind::BufferB) {
            if buffer_b_src.trim().is_empty() {
                log::debug!("BufferB is empty, skipping");
                None
            } else {
                let has_code = buffer_b_src.contains("fn fs_main") || buffer_b_src.contains("@fragment");
                if !has_code {
                    log::debug!("BufferB has no fragment shader code, skipping");
                    None
                } else {
                    log::debug!("Creating BufferB pass");
                    if let Err(e) = validate_shader(buffer_b_src) {
                        log::warn!("[BufferB] Validation failed, skipping: {}", e);
                        None
                    } else {
                        let buffer_b_module = device.create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
                            label: Some("buffer_b_shader"),
                            source: eframe::wgpu::ShaderSource::Wgsl(buffer_b_src.clone().into()),
                        });
                        let (buffer_b_tex, buffer_b_view) =
                            create_color_target(device, screen_size, format, "buffer_b_target");
                        let buffer_b_pipeline_layout =
                            device.create_pipeline_layout(&eframe::wgpu::PipelineLayoutDescriptor {
                                label: Some("buffer_b_pipeline_layout"),
                                bind_group_layouts: &[&uniform_bgl],
                                push_constant_ranges: &[],
                            });
                        let buffer_b_pipeline =
                            device.create_render_pipeline(&eframe::wgpu::RenderPipelineDescriptor {
                                label: Some("buffer_b_pipeline"),
                                layout: Some(&buffer_b_pipeline_layout),
                                vertex: eframe::wgpu::VertexState {
                                    module: &buffer_b_module,
                                    entry_point: Some("vs_main"),
                                    compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
                                    buffers: &[],
                                },
                                fragment: Some(eframe::wgpu::FragmentState {
                                    module: &buffer_b_module,
                                    entry_point: Some("fs_main"),
                                    compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
                                    targets: &[Some(eframe::wgpu::ColorTargetState {
                                        format,
                                        blend: Some(eframe::wgpu::BlendState::ALPHA_BLENDING),
                                        write_mask: eframe::wgpu::ColorWrites::ALL,
                                    })],
                                }),
                                primitive: eframe::wgpu::PrimitiveState::default(),
                                depth_stencil: None,
                                multisample: eframe::wgpu::MultisampleState::default(),
                                multiview: None,
                                cache: None,
                            });
                        Some(BufferPass {
                            kind: BufferKind::BufferB,
                            pipeline: buffer_b_pipeline,
                            target_texture: buffer_b_tex,
                            target_view: buffer_b_view,
                        })
                    }
                }
            }
        } else {
            None
        };

        // ===== BUFFER C: offscreen pass (optional) =====
        let buffer_c = if let Some(buffer_c_src) = sources.get(&BufferKind::BufferC) {
            if buffer_c_src.trim().is_empty() {
                log::debug!("BufferC is empty, skipping");
                None
            } else {
                let has_code = buffer_c_src.contains("fn fs_main") || buffer_c_src.contains("@fragment");
                if !has_code {
                    log::debug!("BufferC has no fragment shader code, skipping");
                    None
                } else {
                    log::debug!("Creating BufferC pass");
                    if let Err(e) = validate_shader(buffer_c_src) {
                        log::warn!("[BufferC] Validation failed, skipping: {}", e);
                        None
                    } else {
                        let buffer_c_module = device.create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
                            label: Some("buffer_c_shader"),
                            source: eframe::wgpu::ShaderSource::Wgsl(buffer_c_src.clone().into()),
                        });
                        let (buffer_c_tex, buffer_c_view) =
                            create_color_target(device, screen_size, format, "buffer_c_target");
                        let buffer_c_pipeline_layout =
                            device.create_pipeline_layout(&eframe::wgpu::PipelineLayoutDescriptor {
                                label: Some("buffer_c_pipeline_layout"),
                                bind_group_layouts: &[&uniform_bgl],
                                push_constant_ranges: &[],
                            });
                        let buffer_c_pipeline =
                            device.create_render_pipeline(&eframe::wgpu::RenderPipelineDescriptor {
                                label: Some("buffer_c_pipeline"),
                                layout: Some(&buffer_c_pipeline_layout),
                                vertex: eframe::wgpu::VertexState {
                                    module: &buffer_c_module,
                                    entry_point: Some("vs_main"),
                                    compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
                                    buffers: &[],
                                },
                                fragment: Some(eframe::wgpu::FragmentState {
                                    module: &buffer_c_module,
                                    entry_point: Some("fs_main"),
                                    compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
                                    targets: &[Some(eframe::wgpu::ColorTargetState {
                                        format,
                                        blend: Some(eframe::wgpu::BlendState::ALPHA_BLENDING),
                                        write_mask: eframe::wgpu::ColorWrites::ALL,
                                    })],
                                }),
                                primitive: eframe::wgpu::PrimitiveState::default(),
                                depth_stencil: None,
                                multisample: eframe::wgpu::MultisampleState::default(),
                                multiview: None,
                                cache: None,
                            });
                        Some(BufferPass {
                            kind: BufferKind::BufferC,
                            pipeline: buffer_c_pipeline,
                            target_texture: buffer_c_tex,
                            target_view: buffer_c_view,
                        })
                    }
                }
            }
        } else {
            None
        };

        // ===== BUFFER D: offscreen pass (optional) =====
        let buffer_d = if let Some(buffer_d_src) = sources.get(&BufferKind::BufferD) {
            if buffer_d_src.trim().is_empty() {
                log::debug!("BufferD is empty, skipping");
                None
            } else {
                let has_code = buffer_d_src.contains("fn fs_main") || buffer_d_src.contains("@fragment");
                if !has_code {
                    log::debug!("BufferD has no fragment shader code, skipping");
                    None
                } else {
                    log::debug!("Creating BufferD pass");
                    if let Err(e) = validate_shader(buffer_d_src) {
                        log::warn!("[BufferD] Validation failed, skipping: {}", e);
                        None
                    } else {
                        let buffer_d_module = device.create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
                            label: Some("buffer_d_shader"),
                            source: eframe::wgpu::ShaderSource::Wgsl(buffer_d_src.clone().into()),
                        });
                        let (buffer_d_tex, buffer_d_view) =
                            create_color_target(device, screen_size, format, "buffer_d_target");
                        let buffer_d_pipeline_layout =
                            device.create_pipeline_layout(&eframe::wgpu::PipelineLayoutDescriptor {
                                label: Some("buffer_d_pipeline_layout"),
                                bind_group_layouts: &[&uniform_bgl],
                                push_constant_ranges: &[],
                            });
                        let buffer_d_pipeline =
                            device.create_render_pipeline(&eframe::wgpu::RenderPipelineDescriptor {
                                label: Some("buffer_d_pipeline"),
                                layout: Some(&buffer_d_pipeline_layout),
                                vertex: eframe::wgpu::VertexState {
                                    module: &buffer_d_module,
                                    entry_point: Some("vs_main"),
                                    compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
                                    buffers: &[],
                                },
                                fragment: Some(eframe::wgpu::FragmentState {
                                    module: &buffer_d_module,
                                    entry_point: Some("fs_main"),
                                    compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
                                    targets: &[Some(eframe::wgpu::ColorTargetState {
                                        format,
                                        blend: Some(eframe::wgpu::BlendState::ALPHA_BLENDING),
                                        write_mask: eframe::wgpu::ColorWrites::ALL,
                                    })],
                                }),
                                primitive: eframe::wgpu::PrimitiveState::default(),
                                depth_stencil: None,
                                multisample: eframe::wgpu::MultisampleState::default(),
                                multiview: None,
                                cache: None,
                            });
                        Some(BufferPass {
                            kind: BufferKind::BufferD,
                            pipeline: buffer_d_pipeline,
                            target_texture: buffer_d_tex,
                            target_view: buffer_d_view,
                        })
                    }
                }
            }
        } else {
            None
        };

        // ===== MAIN IMAGE: reads BufferA texture =====
        let main_src = sources
            .get(&BufferKind::MainImage)
            .ok_or_else(|| ShaderError::CompilationError("[MainImage] Missing shader source".into()))?;

        // Skip if empty
        if main_src.trim().is_empty() {
            return Err(ShaderError::CompilationError("[MainImage] Shader source is empty".into()));
        }

        log::debug!("Creating MainImage pipeline");
        validate_shader(main_src)
            .map_err(|e| ShaderError::CompilationError(format!("[MainImage] {}", e)))?;

        let main_module = device.create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
            label: Some("main_image_shader"),
            source: eframe::wgpu::ShaderSource::Wgsl(main_src.clone().into()),
        });

        let main_pipeline_layout =
            device.create_pipeline_layout(&eframe::wgpu::PipelineLayoutDescriptor {
                label: Some("main_pipeline_layout"),
                bind_group_layouts: &[&uniform_bgl, &texture_bgl],
                push_constant_ranges: &[],
            });

        let main_pipeline =
            device.create_render_pipeline(&eframe::wgpu::RenderPipelineDescriptor {
                label: Some("main_image_pipeline"),
                layout: Some(&main_pipeline_layout),
                vertex: eframe::wgpu::VertexState {
                    module: &main_module,
                    entry_point: Some("vs_main"),
                    compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(eframe::wgpu::FragmentState {
                    module: &main_module,
                    entry_point: Some("fs_main"),
                    compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(eframe::wgpu::ColorTargetState {
                        format,
                        blend: Some(eframe::wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: eframe::wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: eframe::wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: eframe::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // ===== Bind group for MainImage to read all buffer textures =====
        // Create dummy texture for any missing buffers
        let (_dummy_tex, dummy_view) = create_color_target(device, [1, 1], format, "dummy_texture");
        
        let main_tex_bg = device.create_bind_group(&eframe::wgpu::BindGroupDescriptor {
            label: Some("main_texture_bg"),
            layout: &texture_bgl,
            entries: &[
                // BufferA @binding(0)
                eframe::wgpu::BindGroupEntry {
                    binding: 0,
                    resource: eframe::wgpu::BindingResource::TextureView(
                        if let Some(ref ba) = buffer_a { &ba.target_view } else { &dummy_view }
                    ),
                },
                // Sampler A @binding(1)
                eframe::wgpu::BindGroupEntry {
                    binding: 1,
                    resource: eframe::wgpu::BindingResource::Sampler(&sampler),
                },
                // BufferB @binding(2)
                eframe::wgpu::BindGroupEntry {
                    binding: 2,
                    resource: eframe::wgpu::BindingResource::TextureView(
                        if let Some(ref bb) = buffer_b { &bb.target_view } else { &dummy_view }
                    ),
                },
                // Sampler B @binding(3)
                eframe::wgpu::BindGroupEntry {
                    binding: 3,
                    resource: eframe::wgpu::BindingResource::Sampler(&sampler),
                },
                // BufferC @binding(4)
                eframe::wgpu::BindGroupEntry {
                    binding: 4,
                    resource: eframe::wgpu::BindingResource::TextureView(
                        if let Some(ref bc) = buffer_c { &bc.target_view } else { &dummy_view }
                    ),
                },
                // Sampler C @binding(5)
                eframe::wgpu::BindGroupEntry {
                    binding: 5,
                    resource: eframe::wgpu::BindingResource::Sampler(&sampler),
                },
                // BufferD @binding(6)
                eframe::wgpu::BindGroupEntry {
                    binding: 6,
                    resource: eframe::wgpu::BindingResource::TextureView(
                        if let Some(ref bd) = buffer_d { &bd.target_view } else { &dummy_view }
                    ),
                },
                // Sampler D @binding(7)
                eframe::wgpu::BindGroupEntry {
                    binding: 7,
                    resource: eframe::wgpu::BindingResource::Sampler(&sampler),
                },
                // User image texture iChannel0 @binding(8)
                eframe::wgpu::BindGroupEntry {
                    binding: 8,
                    resource: eframe::wgpu::BindingResource::TextureView(
                        user_image_views[0].as_ref().unwrap_or(&dummy_view)
                    ),
                },
                // User image sampler iChannel0 @binding(9)
                eframe::wgpu::BindGroupEntry {
                    binding: 9,
                    resource: eframe::wgpu::BindingResource::Sampler(&sampler),
                },
                // User image texture iChannel1 @binding(10)
                eframe::wgpu::BindGroupEntry {
                    binding: 10,
                    resource: eframe::wgpu::BindingResource::TextureView(
                        user_image_views[1].as_ref().unwrap_or(&dummy_view)
                    ),
                },
                // User image sampler iChannel1 @binding(11)
                eframe::wgpu::BindGroupEntry {
                    binding: 11,
                    resource: eframe::wgpu::BindingResource::Sampler(&sampler),
                },
                // User image texture iChannel2 @binding(12)
                eframe::wgpu::BindGroupEntry {
                    binding: 12,
                    resource: eframe::wgpu::BindingResource::TextureView(
                        user_image_views[2].as_ref().unwrap_or(&dummy_view)
                    ),
                },
                // User image sampler iChannel2 @binding(13)
                eframe::wgpu::BindGroupEntry {
                    binding: 13,
                    resource: eframe::wgpu::BindingResource::Sampler(&sampler),
                },
                // User image texture iChannel3 @binding(14)
                eframe::wgpu::BindGroupEntry {
                    binding: 14,
                    resource: eframe::wgpu::BindingResource::TextureView(
                        user_image_views[3].as_ref().unwrap_or(&dummy_view)
                    ),
                },
                // User image sampler iChannel3 @binding(15)
                eframe::wgpu::BindGroupEntry {
                    binding: 15,
                    resource: eframe::wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        log::info!("Multi-pass shader pipeline created successfully");

        Ok(Self {
            uniform_buffer,
            uniform_bind_group_layout: uniform_bgl,
            texture_bind_group_layout: texture_bgl,
            uniform_bind_group: uniform_bg,
            buffer_a,
            buffer_b,
            buffer_c,
            buffer_d,
            main_image_pipeline: main_pipeline,
            main_texture_bind_group: main_tex_bg,
            sampler,
            start_time: Instant::now(),
            user_image_textures,
            user_image_views,
        })
    }

    /// Record all render passes: buffers first, then main image
    pub fn record_passes(&self, encoder: &mut CommandEncoder, screen_view: &TextureView) {
        // 1) Buffer A → its own texture
        if let Some(ref buffer_a) = self.buffer_a {
            buffer_a.render(encoder, &self.uniform_bind_group);
        }

        // 2) Buffer B, C, D (when implemented)
        // ...

        // 3) MainImage → screen, sampling from BufferA.target_view
        {
            let mut rpass = encoder.begin_render_pass(&eframe::wgpu::RenderPassDescriptor {
                label: Some("main_image_pass"),
                color_attachments: &[Some(eframe::wgpu::RenderPassColorAttachment {
                    view: screen_view,
                    resolve_target: None,
                    ops: eframe::wgpu::Operations {
                        load: eframe::wgpu::LoadOp::Clear(eframe::wgpu::Color::BLACK),
                        store: eframe::wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.main_image_pipeline);
            rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
            rpass.set_bind_group(1, &self.main_texture_bind_group, &[]);
            rpass.draw(0..6, 0..1);
        }
    }

    /// Update uniforms before rendering
    pub fn update_uniforms(&self, queue: &Queue, uniforms: &ShaderUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }
}

/// Callback for rendering multi-pass shader
pub struct MultiPassCallback {
    pub shader: Arc<MultiPassPipelines>,
    pub bass_energy: Arc<std::sync::Mutex<f32>>,
    pub mid_energy: Arc<std::sync::Mutex<f32>>,
    pub high_energy: Arc<std::sync::Mutex<f32>>,
}

impl eframe::egui_wgpu::CallbackTrait for MultiPassCallback {
    fn prepare(
        &self,
        _device: &eframe::wgpu::Device,
        queue: &eframe::wgpu::Queue,
        screen_descriptor: &eframe::egui_wgpu::ScreenDescriptor,
        encoder: &mut eframe::wgpu::CommandEncoder,
        _resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<eframe::wgpu::CommandBuffer> {
        let elapsed = self.shader.start_time.elapsed().as_secs_f32();

        let resolution = [
            screen_descriptor.size_in_pixels[0] as f32,
            screen_descriptor.size_in_pixels[1] as f32,
        ];

        let bass = *self.bass_energy.lock().unwrap();
        let mid = *self.mid_energy.lock().unwrap();
        let high = *self.high_energy.lock().unwrap();

        let uniforms = ShaderUniforms {
            time: elapsed,
            audio_bass: bass,
            audio_mid: mid,
            audio_high: high,
            resolution,
            _pad0: [0.0, 0.0],
        };

        self.shader.update_uniforms(queue, &uniforms);

        // Render buffer passes to offscreen textures
        if let Some(ref buffer_a) = self.shader.buffer_a {
            buffer_a.render(encoder, &self.shader.uniform_bind_group);
        }
        if let Some(ref buffer_b) = self.shader.buffer_b {
            buffer_b.render(encoder, &self.shader.uniform_bind_group);
        }
        if let Some(ref buffer_c) = self.shader.buffer_c {
            buffer_c.render(encoder, &self.shader.uniform_bind_group);
        }
        if let Some(ref buffer_d) = self.shader.buffer_d {
            buffer_d.render(encoder, &self.shader.uniform_bind_group);
        }

        Vec::new()
    }

    fn paint(
        &self,
        _info: epaint::PaintCallbackInfo,
        render_pass: &mut eframe::wgpu::RenderPass<'static>,
        _resources: &eframe::egui_wgpu::CallbackResources,
    ) {
        // Render main image (which samples from buffer textures)
        render_pass.set_pipeline(&self.shader.main_image_pipeline);
        render_pass.set_bind_group(0, &self.shader.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &self.shader.main_texture_bind_group, &[]);
        render_pass.draw(0..6, 0..1);

        static FIRST_RENDER: std::sync::Once = std::sync::Once::new();
        FIRST_RENDER.call_once(|| {
            log::debug!("First multi-pass shader render executed");
        });
    }
}
