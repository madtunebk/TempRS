// Test unit for Universe Within shader
// Loads and displays the shader from ideas/shard.txt (ported to WGSL)

use eframe::egui;
use std::sync::Arc;
use std::time::Instant;
use std::env;
use std::fs;
use egui_wgpu::wgpu::{Device, Queue, RenderPipeline, Buffer, BindGroup};

// Shader uniforms structure
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderUniforms {
    pub time: f32,
    pub audio_bass: f32,
    pub audio_mid: f32,
    pub audio_high: f32,
    pub resolution: [f32; 2],
    pub _pad0: [f32; 2],
}

// Shader pipeline wrapper
pub struct ShaderPipeline {
    pub pipeline: RenderPipeline,
    pub uniform_buffer: Buffer,
    pub bind_group: BindGroup,
    pub start_time: Instant,
}

impl ShaderPipeline {
    pub fn new(device: &Device, format: egui_wgpu::wgpu::TextureFormat, wgsl_src: &str) -> Self {
        log::debug!("[ShaderPipeline] Creating shader module from WGSL source ({} bytes)", wgsl_src.len());
        let shader = device.create_shader_module(egui_wgpu::wgpu::ShaderModuleDescriptor {
            label: Some("universe_within_shader"),
            source: egui_wgpu::wgpu::ShaderSource::Wgsl(wgsl_src.into()),
        });
        log::debug!("[ShaderPipeline] Shader module created successfully");

        let uniform_size = std::mem::size_of::<ShaderUniforms>() as u64;
        log::debug!("[ShaderPipeline] Creating uniform buffer (size: {} bytes)", uniform_size);
        let uniform_buffer = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("shader_uniforms"),
            size: uniform_size,
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

        log::info!("[ShaderPipeline] Pipeline created successfully with format: {:?}", format);
        
        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            start_time: Instant::now(),
        }
    }
}

// Callback for rendering shader
pub struct ShaderCallback {
    pub shader: Arc<ShaderPipeline>,
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

        let uniforms = ShaderUniforms {
            time: elapsed,
            audio_bass: 0.0,
            audio_mid: 0.0,
            audio_high: 0.0,
            resolution: [
                screen_descriptor.size_in_pixels[0] as f32,
                screen_descriptor.size_in_pixels[1] as f32,
            ],
            _pad0: [0.0, 0.0],
        };

        // Log every 3 seconds to reduce overhead
        if elapsed as u32 % 3 == 0 && (elapsed * 10.0) as u32 % 10 == 0 {
            log::debug!(
                "[ShaderCallback] Frame update - time: {:.2}s, resolution: {}x{}, FPS: ~30",
                elapsed,
                uniforms.resolution[0],
                uniforms.resolution[1]
            );
        }

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
        
        // Log first render only
        static FIRST_RENDER: std::sync::Once = std::sync::Once::new();
        FIRST_RENDER.call_once(|| {
            log::info!("[ShaderCallback] First render executed - drawing 6 vertices");
        });
    }
}

// Main app structure
struct ShaderTestApp {
    shader: Option<Arc<ShaderPipeline>>,
    shader_path: String,
}

impl Default for ShaderTestApp {
    fn default() -> Self {
        Self {
            shader: None,
            shader_path: String::new(),
        }
    }
}

impl ShaderTestApp {
    /// Create new app with shader initialized from eframe CreationContext
    pub fn new(cc: &eframe::CreationContext<'_>, shader_path: String) -> Self {
        let mut app = Self::default();
        app.shader_path = shader_path.clone();
        
        // Initialize shader pipeline if WGPU is available
        if let Some(render_state) = cc.wgpu_render_state.as_ref() {
            log::debug!("[ShaderTestApp] Render state available in CreationContext");
            log::info!("[ShaderTestApp] Loading WGSL shader from: {}", shader_path);
            
            // Read shader from file or use default
            let wgsl_source = if shader_path.is_empty() {
                log::info!("[ShaderTestApp] No shader specified, using default");
                include_str!("../src/shaders/converted_test.wgsl").to_string()
            } else {
                fs::read_to_string(&shader_path).unwrap_or_else(|e| {
                    log::error!("[ShaderTestApp] Failed to read shader file '{}': {}", shader_path, e);
                    log::info!("[ShaderTestApp] Falling back to default shader");
                    include_str!("../src/shaders/converted_test.wgsl").to_string()
                })
            };
            
            log::debug!("[ShaderTestApp] Creating pipeline with target format: {:?}", render_state.target_format);
            
            let pipeline = ShaderPipeline::new(
                &render_state.device,
                render_state.target_format,
                &wgsl_source,
            );
            app.shader = Some(Arc::new(pipeline));
            log::info!("✨ [ShaderTestApp] Universe Within shader loaded and ready!");
        } else {
            log::error!("[ShaderTestApp] WGPU render state not available in CreationContext!");
        }
        
        app
    }
}

impl eframe::App for ShaderTestApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Limit to 30 FPS for smoother performance
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
        
        egui::CentralPanel::default().show(ctx, |ui| {
            let screen_rect = ui.max_rect();
            
            // Render the shader
            if let Some(shader) = &self.shader {
                let rect = screen_rect;
                
                let callback = egui_wgpu::Callback::new_paint_callback(
                    rect,
                    ShaderCallback {
                        shader: Arc::clone(shader),
                    },
                );
                
                ui.painter().add(callback);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Initializing shader...");
                });
            }
            
            // Request continuous repaint for animation
            ctx.request_repaint();
        });
    }
}

fn main() -> eframe::Result<()> {
    // Initialize logger with info level (use RUST_LOG=debug for verbose output)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .filter_module("wgpu_hal", log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Warn)
        .filter_module("winit", log::LevelFilter::Warn)
        .filter_module("eframe", log::LevelFilter::Info)
        .init();
    
    // Get shader path from command line argument
    let args: Vec<String> = env::args().collect();
    let shader_path = if args.len() > 1 {
        args[1].clone()
    } else {
        log::info!("Usage: shader_test <shader.wgsl>");
        log::info!("No shader specified, using default converted_test.wgsl");
        String::new()
    };
    
    log::info!("🚀 Starting shader test...");
    if !shader_path.is_empty() {
        log::info!("📄 Testing shader: {}", shader_path);
    } else {
        log::info!("📄 Using default shader: src/shaders/converted_test.wgsl");
    }
    log::info!("📊 Window size: 1280x720");
    log::info!("🎨 Renderer: WGPU");
    
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Universe Within - Shader Test")
            .with_resizable(true),
        renderer: eframe::Renderer::Wgpu,
        vsync: true,  // Enable vsync for smoother frame pacing
        multisampling: 0,  // Disable MSAA for better performance
        ..Default::default()
    };
    
    log::debug!("[Main] Launching eframe application...");
    let result = eframe::run_native(
        "Shader Test",
        native_options,
        Box::new(move |cc| {
            log::debug!("[Main] App creation callback invoked");
            Ok(Box::new(ShaderTestApp::new(cc, shader_path)))
        }),
    );
    
    log::info!("👋 Shader test application exited");
    result
}
