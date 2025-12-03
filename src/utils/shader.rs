use std::sync::Arc;
use std::time::Instant;

use egui::{self};
use egui_wgpu::wgpu::{Device, Queue, RenderPipeline, Buffer, BindGroup};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderUniforms {
    pub time: f32,
    pub audio_bass: f32,      // Low frequencies (0-250Hz)
    pub audio_mid: f32,       // Mid frequencies (250-2000Hz)
    pub audio_high: f32,      // High frequencies (2000Hz+)
    pub resolution: [f32; 2],
    pub _pad0: [f32; 2],      // Padding to 32 bytes
}

pub struct ShaderPipeline {
    pub pipeline: RenderPipeline,
    pub uniform_buffer: Buffer,
    pub bind_group: BindGroup,
    pub start_time: Instant,
}

impl ShaderPipeline {
    pub fn new(device: &Device, format: egui_wgpu::wgpu::TextureFormat, wgsl_src: &str) -> Self {
        let shader = device.create_shader_module(egui_wgpu::wgpu::ShaderModuleDescriptor {
            label: Some("dynamic shader"),
            source: egui_wgpu::wgpu::ShaderSource::Wgsl(wgsl_src.into()),
        });

        let uniform_buffer = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("shader_uniforms"),
            size: std::mem::size_of::<ShaderUniforms>() as u64,
            usage: egui_wgpu::wgpu::BufferUsages::COPY_DST | egui_wgpu::wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let uniform_bgl = device.create_bind_group_layout(&egui_wgpu::wgpu::BindGroupLayoutDescriptor {
            label: Some("shader_bgl"),
            entries: &[egui_wgpu::wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: egui_wgpu::wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: egui_wgpu::wgpu::BindingType::Buffer {
                    ty: egui_wgpu::wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
            label: Some("shader_bg"),
            layout: &uniform_bgl,
            entries: &[egui_wgpu::wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("shader_pipeline_layout"),
                bind_group_layouts: &[&uniform_bgl],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
            label: Some("shader_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: egui_wgpu::wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(egui_wgpu::wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                    format,
                    blend: Some(egui_wgpu::wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: egui_wgpu::wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: egui_wgpu::wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: egui_wgpu::wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            start_time: Instant::now(),
        }
    }
}

pub struct ShaderCallback {
    pub shader: Arc<ShaderPipeline>,
    pub audio_bass: f32,   // Real FFT bass energy (0.0 - 1.0)
    pub audio_mid: f32,    // Real FFT mid energy (0.0 - 1.0)
    pub audio_high: f32,   // Real FFT high energy (0.0 - 1.0)
}

impl egui_wgpu::CallbackTrait for ShaderCallback {
    fn prepare(
        &self,
        _device: &Device,
        queue: &Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut egui_wgpu::wgpu::CommandEncoder,
        _callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<egui_wgpu::wgpu::CommandBuffer> {
        let elapsed = self.shader.start_time.elapsed().as_secs_f32();

        // Use REAL FFT data - already normalized 0.0-1.0
        // Amplify slightly for better visual impact
        let bass = self.audio_bass * 1.5;
        let mid = self.audio_mid * 1.3;
        let high = self.audio_high * 1.2;

        let uniforms = ShaderUniforms {
            time: elapsed,
            audio_bass: bass,
            audio_mid: mid,
            audio_high: high,
            resolution: [
                screen_descriptor.size_in_pixels[0] as f32,
                screen_descriptor.size_in_pixels[1] as f32,
            ],
            _pad0: [0.0, 0.0],
        };

        queue.write_buffer(&self.shader.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        Vec::new()
    }

    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut egui_wgpu::wgpu::RenderPass<'static>,
        _callback_resources: &'a egui_wgpu::CallbackResources,
    ) {
        render_pass.set_pipeline(&self.shader.pipeline);
        render_pass.set_bind_group(0, &self.shader.bind_group, &[]);
        render_pass.draw(0..6, 0..1);  // Draw 6 vertices (2 triangles)
    }
}
