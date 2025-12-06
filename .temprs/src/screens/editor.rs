use eframe::egui;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::screens::shader_buffer::ShaderBuffer;
use crate::ui_components::{settings_menu, shader_properties};
use crate::utils::{
    catch_panic_mut, format_panic_message, format_shader_error, validate_shader, BufferKind,
    MultiPassCallback, MultiPassPipelines, NotificationManager, ShaderError, ShaderJson,
    DEFAULT_FONT_SIZE, DEFAULT_FRAGMENT, DEFAULT_VERTEX, SHADER_BOILERPLATE, STANDARD_VERTEX,
    DEFAULT_BUFFER_RESOLUTION,
};

pub struct TopApp {
    // Unified buffer system - single HashMap instead of 5 separate fields
    buffers: HashMap<BufferKind, ShaderBuffer>,
    current_buffer: BufferKind,

    saved_shaders: Option<HashMap<BufferKind, (String, String)>>,

    // Multi-pass shader pipeline
    shader_shared: Arc<Mutex<Option<Arc<MultiPassPipelines>>>>,
    shader_needs_update: Arc<AtomicBool>,
    last_error: Arc<Mutex<Option<ShaderError>>>,
    target_format: Option<egui_wgpu::wgpu::TextureFormat>,

    // UI state
    editor_font_size: f32,
    show_settings: bool,
    show_error_window: bool,
    error_message: String,
    show_preset_menu: bool,

    // Audio - FFT energy values
    bass_energy: Arc<Mutex<f32>>,
    mid_energy: Arc<Mutex<f32>>,
    high_energy: Arc<Mutex<f32>>,
    debug_audio: bool,
    debug_bass: f32,
    debug_mid: f32,
    debug_high: f32,
    audio_file_path: Option<String>,
    image_file_paths: [Option<String>; 4], // Support up to 4 image channels (iChannel0-3)
    selected_image_channel: usize, // Which channel to load next image into (0-3)

    // Notifications
    notification_mgr: NotificationManager,

    // Preview overlay state
    _preview_overlay_hovered: bool,
}

impl TopApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        log::info!("Initializing TopApp...");

        // Initialize unified buffer system - single source of truth
        let mut buffers = HashMap::with_capacity(5);

        // Load default preset JSON to get initial buffer content
        let default_json = include_str!("../assets/shards/default.json");
        let default_shader = ShaderJson::from_json(default_json)
            .expect("Failed to parse default.json - this should never fail");

        // MainImage with default shader from JSON
        buffers.insert(
            BufferKind::MainImage,
            ShaderBuffer::new(
                BufferKind::MainImage,
                default_shader.vertex.clone().unwrap_or_else(|| DEFAULT_VERTEX.to_string()),
                default_shader.fragment.clone(),
            ),
        );

        // Buffer A from JSON
        buffers.insert(
            BufferKind::BufferA,
            ShaderBuffer::new(
                BufferKind::BufferA,
                DEFAULT_VERTEX.to_string(),
                default_shader.buffer_a.clone().unwrap_or_else(|| "// Buffer A\n".to_string()),
            ),
        );

        // Buffer B from JSON
        buffers.insert(
            BufferKind::BufferB,
            ShaderBuffer::new(
                BufferKind::BufferB,
                DEFAULT_VERTEX.to_string(),
                default_shader.buffer_b.clone().unwrap_or_else(|| "// Buffer B\n".to_string()),
            ),
        );

        // Buffer C from JSON
        buffers.insert(
            BufferKind::BufferC,
            ShaderBuffer::new(
                BufferKind::BufferC,
                DEFAULT_VERTEX.to_string(),
                default_shader.buffer_c.clone().unwrap_or_else(|| "// Buffer C\n".to_string()),
            ),
        );

        // Buffer D from JSON
        buffers.insert(
            BufferKind::BufferD,
            ShaderBuffer::new(
                BufferKind::BufferD,
                DEFAULT_VERTEX.to_string(),
                default_shader.buffer_d.clone().unwrap_or_else(|| "// Buffer D\n".to_string()),
            ),
        );

        let mut app = Self {
            buffers,
            current_buffer: BufferKind::MainImage,
            saved_shaders: None,

            shader_shared: Arc::new(Mutex::new(None)),
            shader_needs_update: Arc::new(AtomicBool::new(false)),
            last_error: Arc::new(Mutex::new(None)),
            target_format: None,

            editor_font_size: DEFAULT_FONT_SIZE,
            show_settings: false,
            show_error_window: false,
            error_message: String::new(),
            show_preset_menu: false,

            bass_energy: Arc::new(Mutex::new(0.0)),
            mid_energy: Arc::new(Mutex::new(0.0)),
            high_energy: Arc::new(Mutex::new(0.0)),
            debug_audio: false,
            debug_bass: 0.0,
            debug_mid: 0.0,
            debug_high: 0.0,
            audio_file_path: None,
            image_file_paths: [None, None, None, None],
            selected_image_channel: 0,

            notification_mgr: NotificationManager::default(),

            _preview_overlay_hovered: false,
        };

        // Load default shader into MainImage on startup
        app.load_preset_shader("default");

        // Compile initial shader
        if let Some(render_state) = cc.wgpu_render_state.as_ref() {
            let device = render_state.device.clone();
            let queue = render_state.queue.clone();
            let format = render_state.target_format;
            app.target_format = Some(format);

            let sources = app.get_all_buffer_sources();

            match MultiPassPipelines::new(&device, &queue, format, DEFAULT_BUFFER_RESOLUTION, &sources, &app.image_file_paths) {
                Ok(pipeline) => {
                    *app.shader_shared.lock().unwrap() = Some(Arc::new(pipeline));
                }
                Err(err) => {
                    *app.last_error.lock().unwrap() = Some(err);
                }
            }
        }

        log::info!("TopApp initialization complete");
        app
    }
}

impl eframe::App for TopApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Request continuous repainting for smooth audio visualization
        ctx.request_repaint();

        // Apply the theme every frame to prevent visual drift
        crate::utils::apply_editor_theme(ctx);

        // Handle keyboard shortcuts
        self.handle_input(ctx);

        // Handle shader compilation if needed
        if self.shader_needs_update.load(Ordering::Relaxed) {
            log::debug!("Shader update requested, beginning multi-pass compilation");
            self.shader_needs_update.store(false, Ordering::Relaxed);

            if let Some(render_state) = frame.wgpu_render_state() {
                let device = &render_state.device;
                let queue = &render_state.queue;
                let format = render_state.target_format;

                // Build sources map for all buffers with auto-injected boilerplate
                let mut sources = HashMap::with_capacity(5);
                let mut validation_error: Option<ShaderError> = None;

                for buffer_kind in [BufferKind::MainImage, BufferKind::BufferA, BufferKind::BufferB, BufferKind::BufferC, BufferKind::BufferD] {
                    let (vertex, fragment) = self.get_buffer_shaders(buffer_kind);
                    let fragment_trimmed = fragment.trim();

                    // Skip empty fragments (except MainImage which needs default)
                    let has_code = fragment_trimmed.lines()
                        .any(|line| {
                            let trimmed_line = line.trim();
                            !trimmed_line.is_empty() && !trimmed_line.starts_with("//")
                        });

                    if fragment_trimmed.is_empty() || !has_code {
                        if buffer_kind == BufferKind::MainImage {
                            // MainImage must exist, use default
                            sources.insert(buffer_kind, format!("{}\n{}\n{}", SHADER_BOILERPLATE, STANDARD_VERTEX, DEFAULT_FRAGMENT));
                        }
                        continue;
                    }

                    // Auto-inject boilerplate + standard vertex unless user provides custom vertex
                    let vertex_trimmed = vertex.trim();
                    let user_vertex = if vertex_trimmed.is_empty() || vertex_trimmed == DEFAULT_VERTEX.trim() {
                        STANDARD_VERTEX
                    } else {
                        vertex_trimmed
                    };

                    // Build complete shader with conditional texture bindings
                    // MainImage gets texture bindings to sample from buffers A-D
                    // Buffer A-D do NOT get texture bindings (they're independent)
                    let needs_textures = buffer_kind == BufferKind::MainImage;

                    let mut complete_shader = String::with_capacity(
                        SHADER_BOILERPLATE.len() + user_vertex.len() + fragment_trimmed.len() + 200
                    );
                    complete_shader.push_str(SHADER_BOILERPLATE);

                    // Add texture bindings ONLY for MainImage
                    // Layout matches multi_buffer_pipeline.rs bind group layout:
                    // Buffer A: texture @0, sampler @1
                    // Buffer B: texture @2, sampler @3
                    // Buffer C: texture @4, sampler @5
                    // Buffer D: texture @6, sampler @7
                    if needs_textures {
                        complete_shader.push_str("\n// Multi-pass texture bindings\n");
                        complete_shader.push_str("@group(1) @binding(0) var buffer_a_texture: texture_2d<f32>;\n");
                        complete_shader.push_str("@group(1) @binding(1) var buffer_a_sampler: sampler;\n");
                        complete_shader.push_str("@group(1) @binding(2) var buffer_b_texture: texture_2d<f32>;\n");
                        complete_shader.push_str("@group(1) @binding(3) var buffer_b_sampler: sampler;\n");
                        complete_shader.push_str("@group(1) @binding(4) var buffer_c_texture: texture_2d<f32>;\n");
                        complete_shader.push_str("@group(1) @binding(5) var buffer_c_sampler: sampler;\n");
                        complete_shader.push_str("@group(1) @binding(6) var buffer_d_texture: texture_2d<f32>;\n");
                        complete_shader.push_str("@group(1) @binding(7) var buffer_d_sampler: sampler;\n");
                    }

                    complete_shader.push('\n');
                    complete_shader.push_str(user_vertex);
                    complete_shader.push('\n');
                    complete_shader.push_str(fragment_trimmed);

                    // Validate the complete shader
                    if let Err(e) = validate_shader(&complete_shader) {
                        validation_error = Some(ShaderError::ValidationError(
                            format!("[{:?}] {}", buffer_kind, e)
                        ));
                        break;
                    }

                    sources.insert(buffer_kind, complete_shader);
                }

                // Ensure MainImage exists
                sources.entry(BufferKind::MainImage).or_insert_with(|| {
                    format!("{}\n{}\n{}", SHADER_BOILERPLATE, STANDARD_VERTEX, DEFAULT_FRAGMENT)
                });

                // Check if validation failed
                if let Some(err) = validation_error {
                    *self.last_error.lock().unwrap() = Some(err.clone());
                    let formatted = format_shader_error(&err);
                    self.error_message = formatted.clone();
                    self.show_error_window = true;
                    log::error!("[TopApp] Shader validation failed: {}", formatted);
                    return;
                }

                log::debug!("[TopApp] Compiling multi-pass pipeline with {} buffers", sources.len());

                // Wrap pipeline creation in panic catcher to prevent crashes
                // Use catch_panic_mut since WGPU Device is not UnwindSafe
                let result = catch_panic_mut(|| {
                    MultiPassPipelines::new(device, queue, format, DEFAULT_BUFFER_RESOLUTION, &sources, &self.image_file_paths)
                });

                match result {
                    Ok(Ok(pipeline)) => {
                        // Success: pipeline created
                        *self.shader_shared.lock().unwrap() = Some(Arc::new(pipeline));
                        *self.last_error.lock().unwrap() = None;
                        self.notification_mgr.dismiss_all();
                        self.notification_mgr.success("Multi-pass shader compiled successfully!");
                        log::info!("[TopApp] Multi-pass shader compiled successfully");
                    }
                    Ok(Err(err)) => {
                        // Expected error: shader compilation/validation error
                        *self.last_error.lock().unwrap() = Some(err.clone());
                        let formatted = format_shader_error(&err);

                        self.error_message = formatted.clone();
                        self.show_error_window = true;

                        log::error!("[TopApp] Shader compilation failed: {}", formatted);
                    }
                    Err(panic_msg) => {
                        // Caught panic: WGPU validation error or internal panic
                        let formatted = format_panic_message(&panic_msg);
                        let error = ShaderError::CompilationError(formatted.clone());

                        *self.last_error.lock().unwrap() = Some(error);
                        self.error_message = formatted.clone();
                        self.show_error_window = true;
                        self.notification_mgr.error("Pipeline creation panicked - see error window");

                        log::error!("[TopApp] Pipeline creation panicked: {}", panic_msg);
                    }
                }
            }
        }

        // Main layout: SidePanel (left) + CentralPanel (right)
        egui::SidePanel::left("editor_panel")
            .resizable(false)
            .exact_width(790.0)
            .frame(
                egui::Frame::default()
                    .inner_margin(0.0)
                    .fill(egui::Color32::from_rgb(20, 20, 25)),
            )
            .show(ctx, |ui| {
                self.render_editor_panel(ui, ctx);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_shader_preview(ui);
        });

        // Floating overlays
        let old_font_size = self.editor_font_size;
        settings_menu::settings_overlay(
            ctx,
            &mut self.show_settings,
            &mut self.editor_font_size,
        );
        // Font size changes propagated automatically - no need to update individual tabs
        if (self.editor_font_size - old_font_size).abs() > 0.01 {
            log::debug!("Font size changed to {}", self.editor_font_size);
        }

        // Shader Properties window
        if self.show_preset_menu {
            let action = shader_properties::render(
                ctx,
                &mut self.show_preset_menu,
                &self.audio_file_path,
                &self.image_file_paths,
                &mut self.selected_image_channel,
                &mut self.debug_audio,
                &mut self.debug_bass,
                &mut self.debug_mid,
                &mut self.debug_high,
                &self.bass_energy,
                &self.mid_energy,
                &self.high_energy,
            );

            match action {
                shader_properties::ShaderPropertiesAction::LoadPreset(name) => {
                    self.load_preset_shader(&name);
                }
                shader_properties::ShaderPropertiesAction::LoadAudioFile(path) => {
                    self.load_audio_file(path);
                }
                shader_properties::ShaderPropertiesAction::LoadImageFile(channel, path) => {
                    self.load_image_file(channel, path);
                }
                shader_properties::ShaderPropertiesAction::ExportShard => {
                    self.export_shard();
                }
                shader_properties::ShaderPropertiesAction::ImportShard => {
                    self.import_shard();
                }
                shader_properties::ShaderPropertiesAction::None => {}
            }
        }

        // Toast notifications
        if self.notification_mgr.has_notifications() {
            egui::Window::new("")
                .id(egui::Id::new("toast_notifications_window"))
                .title_bar(false)
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    self.notification_mgr.render(ui);
                });
        }

        // Error window
        if self.show_error_window {
            egui::Window::new("Shader Error")
                .id(egui::Id::new("shader_error_window"))
                .collapsible(false)
                .resizable(true)
                .default_size([600.0, 450.0])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_size(egui::vec2(550.0, 400.0));
                    ui.set_max_size(egui::vec2(800.0, 600.0));

                    egui::ScrollArea::vertical()
                        .id_salt("error_window_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add_space(8.0);
                            ui.style_mut().override_text_style =
                                Some(egui::TextStyle::Monospace);

                            ui.label(
                                egui::RichText::new(&self.error_message)
                                    .color(egui::Color32::from_rgb(255, 120, 120))
                                    .size(13.0)
                                    .family(egui::FontFamily::Monospace),
                            );
                            ui.add_space(10.0);
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() - 70.0);
                        if ui.button("  Close  ").clicked() {
                            self.show_error_window = false;
                        }
                    });
                });
        }
    }
}

impl TopApp {
    fn render_editor_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.spacing_mut().window_margin = egui::Margin::ZERO;

        ui.vertical(|ui| {
            // Buffer tabs
            ui.horizontal(|ui| {
                ui.style_mut().visuals.widgets.inactive.weak_bg_fill =
                    egui::Color32::from_rgb(30, 30, 35);
                ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                    egui::Color32::from_rgb(40, 40, 45);
                ui.style_mut().visuals.widgets.active.weak_bg_fill =
                    egui::Color32::from_rgb(35, 35, 40);

                let tab_h = 36.0;
                let total_tabs = 5.0;
                let tab_width = (ui.available_width() - (total_tabs - 1.0) * 4.0) / total_tabs;

                // Render tabs for all buffers
                for (i, kind) in [BufferKind::MainImage, BufferKind::BufferA, BufferKind::BufferB, BufferKind::BufferC, BufferKind::BufferD].iter().enumerate() {
                    if i > 0 {
                        ui.add_space(4.0);
                    }

                    let is_selected = self.current_buffer == *kind;
                    let button = egui::Button::new(
                        egui::RichText::new(kind.as_str()).size(12.0),
                    )
                    .selected(is_selected)
                    .min_size(egui::vec2(tab_width, tab_h));

                    if ui.add(button).clicked() {
                        self.switch_buffer(*kind);
                    }
                }
            });

            ui.separator();

            // Editor area
            let button_height = 40.0;
            let separator_space = 1.0;
            let reserved = button_height + separator_space;
            let available_height = ui.available_height();

            let _editor_response = ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), available_height - reserved),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    let bg_color = egui::Color32::from_rgb(13, 17, 23);
                    ui.painter().rect_filled(ui.max_rect(), 0.0, bg_color);

                    let editor_rect = ui.max_rect();

                    // Render code editor for current buffer
                    self.render_code_editor(ui);

                    // Floating buttons
                    let gear_size = 32.0;
                    let gear_pos = egui::pos2(
                        editor_rect.right() - gear_size - 8.0,
                        editor_rect.top() + 8.0,
                    );
                    let gear_rect = egui::Rect::from_min_size(
                        gear_pos,
                        egui::vec2(gear_size, gear_size),
                    );

                    let is_hovered =
                        editor_rect.contains(ctx.pointer_hover_pos().unwrap_or_default());
                    if is_hovered || self.show_settings || self.show_preset_menu {
                        // Presets button
                        let preset_pos = egui::pos2(
                            editor_rect.right() - gear_size - 8.0,
                            editor_rect.top() + gear_size + 16.0,
                        );
                        let preset_rect = egui::Rect::from_min_size(
                            preset_pos,
                            egui::vec2(gear_size, gear_size),
                        );

                        let preset_response = ui.put(
                            preset_rect,
                            egui::Button::new(
                                egui::RichText::new("📁").size(16.0),
                            )
                            .frame(true),
                        );

                        if preset_response
                            .on_hover_text("Shader Properties (Presets & Audio)")
                            .clicked()
                        {
                            self.show_preset_menu = !self.show_preset_menu;
                        }

                        // Settings gear button
                        let gear_response = ui.put(
                            gear_rect,
                            egui::Button::new(
                                egui::RichText::new("⚙").size(16.0),
                            )
                            .frame(true),
                        );

                        if gear_response
                            .on_hover_text("Settings (Editor)")
                            .clicked()
                        {
                            self.show_settings = !self.show_settings;
                        }
                    }
                },
            );

            ui.add_space(separator_space);

            // Bottom button bar
            ui.horizontal(|ui| {
                let button_w = (ui.available_width() - 8.0) / 2.0;

                if ui
                    .add_sized(
                        egui::vec2(button_w, button_height),
                        egui::Button::new(
                            egui::RichText::new("⚡ Apply Pipeline")
                                .size(14.0),
                        ),
                    )
                    .on_hover_text("Ctrl+Enter")
                    .clicked()
                {
                    self.apply_shader();
                }

                ui.add_space(8.0);

                if ui
                    .add_sized(
                        egui::vec2(button_w, button_height),
                        egui::Button::new(
                            egui::RichText::new("↻ Reset").size(14.0),
                        ),
                    )
                    .on_hover_text("Reset current tab to its default shader")
                    .clicked()
                {
                    self.reset_shader();
                }
            });
        });
    }

    fn render_code_editor(&mut self, ui: &mut egui::Ui) {
        if let Some(buffer) = self.buffers.get_mut(&self.current_buffer) {
            buffer.render(ui, true, self.editor_font_size);
        }
    }

    fn get_buffer_shaders(&self, buffer: BufferKind) -> (&str, &str) {
        self.buffers
            .get(&buffer)
            .map(|b| b.get_shaders())
            .unwrap_or(("", ""))
    }

    fn get_all_buffer_sources(&self) -> HashMap<BufferKind, String> {
        let mut sources = HashMap::with_capacity(5);
        for buffer_kind in [BufferKind::MainImage, BufferKind::BufferA, BufferKind::BufferB, BufferKind::BufferC, BufferKind::BufferD] {
            let (vertex, fragment) = self.get_buffer_shaders(buffer_kind);
            let combined = format!("{}\n\n{}", vertex.trim(), fragment.trim());
            sources.insert(buffer_kind, combined);
        }
        sources
    }

    fn render_shader_preview(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());

        if let Some(pipeline_arc) = self.shader_shared.lock().unwrap().as_ref() {
            if self.debug_audio {
                *self.bass_energy.lock().unwrap() = self.debug_bass;
                *self.mid_energy.lock().unwrap() = self.debug_mid;
                *self.high_energy.lock().unwrap() = self.debug_high;
            }

            let cb = MultiPassCallback {
                shader: pipeline_arc.clone(),
                bass_energy: self.bass_energy.clone(),
                mid_energy: self.mid_energy.clone(),
                high_energy: self.high_energy.clone(),
            };

            ui.painter()
                .add(egui_wgpu::Callback::new_paint_callback(rect, cb));
        }

        // Overlay controls - show on hover with stable state to prevent flickering
        let pointer_pos = ui.ctx().pointer_hover_pos().unwrap_or_default();
        let preview_hovered = rect.contains(pointer_pos);

        // Calculate overlay rect early to check hover
        let overlay_rect = self.calculate_overlay_rect(rect);
        let overlay_hovered = overlay_rect.contains(pointer_pos);

        // Keep overlay visible if either area is hovered (prevents flicker)
        if preview_hovered || overlay_hovered {
            self.render_preview_overlay(ui, rect);
        }
    }

    fn calculate_overlay_rect(&self, preview_rect: egui::Rect) -> egui::Rect {
        let icon_size = 32.0;  // Match editor button size (gear_size)
        let spacing = 8.0;     // Match spacing between icons

        // Calculate width for 3 icons in a vertical stack (like settings buttons)
        let overlay_width = icon_size;
        let overlay_height = icon_size * 3.0 + spacing * 2.0;

        // Position at bottom-right, matching the top-right settings button position
        let overlay_pos = egui::pos2(
            preview_rect.right() - icon_size - 8.0,  // 8.0 matches gear_pos offset
            preview_rect.bottom() - overlay_height - 8.0,  // 8.0 matches gear_pos offset
        );

        egui::Rect::from_min_size(overlay_pos, egui::vec2(overlay_width, overlay_height))
    }

    fn render_preview_overlay(&mut self, ui: &mut egui::Ui, preview_rect: egui::Rect) {
        // Use helper method for consistent rect calculation
        let overlay_rect = self.calculate_overlay_rect(preview_rect);
        let icon_size = 32.0;
        let spacing = 8.0;

        // No background/border - just floating buttons like settings gear
        // Render icons inside the overlay using vertical layout
        ui.scope_builder(egui::UiBuilder::new().max_rect(overlay_rect), |ui| {
            ui.vertical(|ui| {

                // Icon 1: Load audio file (top)
                let audio_file_pos = egui::pos2(
                    overlay_rect.left(),
                    overlay_rect.top(),
                );
                let audio_file_rect = egui::Rect::from_min_size(
                    audio_file_pos,
                    egui::vec2(icon_size, icon_size),
                );
                let audio_file_response = ui.put(
                    audio_file_rect,
                    egui::Button::new(
                        egui::RichText::new("🎵").size(16.0),
                    )
                    .frame(true),
                );
                if audio_file_response
                    .on_hover_text("Load audio file")
                    .clicked()
                {
                    self.load_audio_file_dialog();
                }

                // Icon 2: Load image (middle)
                let image_pos = egui::pos2(
                    overlay_rect.left(),
                    overlay_rect.top() + icon_size + spacing,
                );
                let image_rect = egui::Rect::from_min_size(
                    image_pos,
                    egui::vec2(icon_size, icon_size),
                );
                let image_response = ui.put(
                    image_rect,
                    egui::Button::new(
                        egui::RichText::new("🖼").size(16.0),
                    )
                    .frame(true),
                );
                if image_response
                    .on_hover_text("Load image texture")
                    .clicked()
                {
                    self.load_image_texture();
                }

                // Icon 3: Audio toggle (bottom)
                let audio_icon = if self.audio_file_path.is_some() { "🔊" } else { "🔇" };
                let audio_tooltip = if self.audio_file_path.is_some() {
                    "Stop audio playback"
                } else {
                    "No audio loaded"
                };
                let audio_pos = egui::pos2(
                    overlay_rect.left(),
                    overlay_rect.top() + (icon_size + spacing) * 2.0,
                );
                let audio_rect = egui::Rect::from_min_size(
                    audio_pos,
                    egui::vec2(icon_size, icon_size),
                );
                let audio_response = ui.put(
                    audio_rect,
                    egui::Button::new(
                        egui::RichText::new(audio_icon).size(16.0),
                    )
                    .frame(true),
                );
                if audio_response.clicked() && self.audio_file_path.is_some() {
                    // Stop audio playback
                    crate::utils::audio_file::stop_audio();
                    self.audio_file_path = None;
                    self.notification_mgr.success("Audio stopped");
                    log::info!("Audio stopped");
                }
                audio_response.on_hover_text(audio_tooltip);
            });
        });
    }

    fn load_image_texture(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image Files", &["png", "jpg", "jpeg", "bmp", "gif", "webp"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            // Use the currently selected channel
            self.load_image_file(self.selected_image_channel, path_str);
        }
    }

    fn load_image_file(&mut self, channel: usize, path: String) {
        if channel > 3 {
            self.notification_mgr.error(format!("Invalid channel: {}", channel));
            return;
        }

        log::info!("Loading image texture to iChannel{}: {}", channel, path);

        // Store the image file path for the selected channel
        self.image_file_paths[channel] = Some(path.clone());

        // Trigger shader recompilation to load the new image texture
        self.shader_needs_update.store(true, Ordering::Relaxed);

        self.notification_mgr.success(format!(
            "Image loaded to iChannel{}: {}",
            channel,
            std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&path)
        ));
        
        log::info!("Image texture available as iChannel{} in all shaders", channel);
    }

    fn load_audio_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Audio Files", &["mp3", "wav", "ogg", "flac"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            log::info!("Loading audio file: {}", path_str);

            // Start audio playback (parameters: bass, mid, high, file_path)
            if crate::utils::audio_file::start_file_audio(
                self.bass_energy.clone(),
                self.mid_energy.clone(),
                self.high_energy.clone(),
                &path_str,
            ).is_some() {
                self.audio_file_path = Some(path_str.clone());
                self.notification_mgr.success(format!("Audio loaded: {}",
                    path.file_name().unwrap_or_default().to_string_lossy()));
            } else {
                self.notification_mgr.error("Failed to load audio file");
            }
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::Plus) {
                self.editor_font_size = (self.editor_font_size + 2.0).min(48.0);
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Minus) {
                self.editor_font_size = (self.editor_font_size - 2.0).max(12.0);
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num0) {
                self.editor_font_size = 16.0;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Enter) {
                self.apply_shader();
            }
            if i.modifiers.command && i.key_pressed(egui::Key::S) {
                self.save_shader_state();
            }
            if i.modifiers.command && i.key_pressed(egui::Key::R) {
                self.restore_shader_state();
            }
            if i.modifiers.command && i.key_pressed(egui::Key::E) {
                self.export_shard();
            }
            if i.modifiers.command && i.key_pressed(egui::Key::I) {
                self.import_shard();
            }
        });
    }

    fn apply_shader(&mut self) {
        log::info!("Apply shader requested - compiling all buffers");
        self.shader_needs_update.store(true, Ordering::Relaxed);
        *self.last_error.lock().unwrap() = None;
        self.notification_mgr.dismiss_all();
    }

    fn reset_shader(&mut self) {
        log::info!("Resetting {} to default shader", self.current_buffer.as_str());

        // Load default preset JSON
        let default_json = include_str!("../assets/shards/default.json");
        let default_shader = match ShaderJson::from_json(default_json) {
            Ok(s) => s,
            Err(e) => {
                self.notification_mgr.error(format!("Failed to load default shader: {}", e));
                return;
            }
        };

        if let Some(buffer) = self.buffers.get_mut(&self.current_buffer) {
            buffer.set_vertex(DEFAULT_VERTEX.to_string());

            let default_fragment = match self.current_buffer {
                BufferKind::MainImage => default_shader.fragment,
                BufferKind::BufferA => default_shader.buffer_a.unwrap_or_else(|| "// Buffer A\n".to_string()),
                BufferKind::BufferB => default_shader.buffer_b.unwrap_or_else(|| "// Buffer B\n".to_string()),
                BufferKind::BufferC => default_shader.buffer_c.unwrap_or_else(|| "// Buffer C\n".to_string()),
                BufferKind::BufferD => default_shader.buffer_d.unwrap_or_else(|| "// Buffer D\n".to_string()),
            };

            buffer.set_fragment(default_fragment.to_string());
        }

        self.apply_shader();
    }

    fn save_shader_state(&mut self) {
        let mut saved = HashMap::new();
        for (kind, buffer) in &self.buffers {
            let (v, f) = buffer.get_shaders();
            saved.insert(*kind, (v.to_string(), f.to_string()));
        }
        self.saved_shaders = Some(saved);
        self.notification_mgr.success("✓ Shader state saved!");
        log::info!("Shader state saved (Ctrl+R to restore)");
    }

    fn restore_shader_state(&mut self) {
        if let Some(saved) = &self.saved_shaders {
            for (kind, (vertex, fragment)) in saved {
                if let Some(buffer) = self.buffers.get_mut(kind) {
                    buffer.set_vertex(vertex.clone());
                    buffer.set_fragment(fragment.clone());
                }
            }
            self.notification_mgr.success("↶ Shader state restored!");
            log::info!("Shader state restored from save point");
        } else {
            self.notification_mgr.error("No saved state available");
            log::warn!("Restore failed: no saved state");
        }
    }

    fn load_audio_file(&mut self, path: String) {
        log::info!("Loading audio file: {}", path);

        // Wrap audio loading in panic catcher to prevent crashes from codec/decoder errors
        let result = catch_panic_mut(|| {
            crate::utils::audio_file::start_file_audio(
                self.bass_energy.clone(),
                self.mid_energy.clone(),
                self.high_energy.clone(),
                &path,
            )
        });

        match result {
            Ok(Some(_)) => {
                self.audio_file_path = Some(path.clone());
                self.notification_mgr.success(format!(
                    "Audio loaded: {}",
                    std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&path)
                ));
                log::info!("Audio playback initialized successfully");
            }
            Ok(None) => {
                self.notification_mgr.error("Failed to load audio file");
                log::warn!("Failed to initialize audio playback from: {}", path);
            }
            Err(panic_msg) => {
                let formatted = format_panic_message(&panic_msg);
                self.notification_mgr.error(format!("Audio loading crashed: {}", formatted));
                log::error!("Audio loading panicked: {}", panic_msg);
            }
        }
    }

    fn switch_buffer(&mut self, new_buffer: BufferKind) {
        if new_buffer == self.current_buffer {
            return;
        }

        self.current_buffer = new_buffer;
        log::info!("Switched to buffer: {:?}", new_buffer);
        self.notification_mgr.info(format!("Switched to {}", new_buffer.as_str()));
    }

    fn load_preset_shader(&mut self, name: &str) {
        // Load JSON preset files instead of individual .frag files
        let json_content = match name {
            "default" => include_str!("../assets/shards/default.json"),
            "psychedelic" => include_str!("../assets/shards/psychedelic.json"),
            "tunnel" => include_str!("../assets/shards/tunnel.json"),
            "raymarch" => include_str!("../assets/shards/raymarch.json"),
            "fractal" => include_str!("../assets/shards/fractal.json"),
            "image_demo" => include_str!("../assets/shards/image_demo.json"),
            _ => {
                self.notification_mgr.error(format!("Unknown preset: {}", name));
                return;
            }
        };

        // Parse and load the JSON shader
        match ShaderJson::from_json(json_content) {
            Ok(shader_json) => {
                self.load_shader_from_json(shader_json);
                self.notification_mgr.success(format!("✓ Loaded preset: {}", name));
                log::info!("Loaded preset shader: {}", name);
            }
            Err(e) => {
                self.notification_mgr.error(format!("Failed to parse preset: {}", e));
                log::error!("Failed to parse preset {}: {}", name, e);
            }
        }
    }

    /// Load a shader from ShaderJson into all buffers
    fn load_shader_from_json(&mut self, shader_json: ShaderJson) {
        // Load MainImage fragment
        if let Some(buffer) = self.buffers.get_mut(&BufferKind::MainImage) {
            buffer.set_fragment(shader_json.fragment.clone());
            if let Some(vertex) = &shader_json.vertex {
                buffer.set_vertex(vertex.clone());
            } else {
                buffer.set_vertex(DEFAULT_VERTEX.to_string());
            }
        }

        // Load Buffer A
        if let Some(buffer_a) = &shader_json.buffer_a {
            if let Some(buffer) = self.buffers.get_mut(&BufferKind::BufferA) {
                buffer.set_fragment(buffer_a.clone());
                buffer.set_vertex(DEFAULT_VERTEX.to_string());
            }
        }

        // Load Buffer B
        if let Some(buffer_b) = &shader_json.buffer_b {
            if let Some(buffer) = self.buffers.get_mut(&BufferKind::BufferB) {
                buffer.set_fragment(buffer_b.clone());
                buffer.set_vertex(DEFAULT_VERTEX.to_string());
            }
        }

        // Load Buffer C
        if let Some(buffer_c) = &shader_json.buffer_c {
            if let Some(buffer) = self.buffers.get_mut(&BufferKind::BufferC) {
                buffer.set_fragment(buffer_c.clone());
                buffer.set_vertex(DEFAULT_VERTEX.to_string());
            }
        }

        // Load Buffer D
        if let Some(buffer_d) = &shader_json.buffer_d {
            if let Some(buffer) = self.buffers.get_mut(&BufferKind::BufferD) {
                buffer.set_fragment(buffer_d.clone());
                buffer.set_vertex(DEFAULT_VERTEX.to_string());
            }
        }

        // Load embedded images from base64
        for (i, channel_data) in [
            &shader_json.ichannel0,
            &shader_json.ichannel1,
            &shader_json.ichannel2,
            &shader_json.ichannel3,
        ].iter().enumerate() {
            if let Some(base64_data) = channel_data {
                // Decode base64 to image bytes
                if let Ok(image_bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data.as_bytes()) {
                    // Save to temporary file in cache
                    if let Some(cache_dir) = dirs::cache_dir() {
                        let temp_dir = cache_dir.join("webshard_editor").join("embedded_textures");
                        let _ = std::fs::create_dir_all(&temp_dir);
                        
                        let temp_path = temp_dir.join(format!("ichannel{}.png", i));
                        if std::fs::write(&temp_path, &image_bytes).is_ok() {
                            self.image_file_paths[i] = Some(temp_path.to_string_lossy().to_string());
                            log::info!("Loaded embedded texture for iChannel{}", i);
                        }
                    }
                }
            }
        }

        // Apply the shader pipeline
        self.apply_shader();
    }

    fn export_shard(&mut self) {
        use std::io::Write;
        use serde_json::json;

        // Default to cache/TempRS/shaders/ folder (where player looks for shaders)
        let cache_shader_dir = dirs::cache_dir()
            .map(|p| p.join("TempRS").join("shaders"))
            .filter(|p| {
                // Create directory if it doesn't exist
                if !p.exists() {
                    let _ = std::fs::create_dir_all(p);
                }
                p.exists()
            });

        let mut dialog = rfd::FileDialog::new()
            .add_filter("JSON Shader", &["json"])
            .set_file_name("shader.json");

        if let Some(dir) = cache_shader_dir {
            dialog = dialog.set_directory(dir);
        }

        let file_path = match dialog.save_file() {
            Some(path) => path,
            None => return,
        };

        // Build JSON object with base64-encoded shaders
        let mut shader_json = json!({
            "version": "1.0",
            "exported_at": chrono::Local::now().to_rfc3339(),
            "encoding": "base64",
        });

        // Get MainImage fragment (required)
        if let Some(buffer) = self.buffers.get(&BufferKind::MainImage) {
            let (_, fragment) = buffer.get_shaders();
            shader_json["fragment"] = json!(ShaderJson::encode_to_base64(fragment));
        } else {
            self.notification_mgr.error("MainImage is required for export");
            return;
        }

        // Add vertex shader if customized (optional)
        if let Some(buffer) = self.buffers.get(&BufferKind::MainImage) {
            let (vertex, _) = buffer.get_shaders();
            let default_vertex = STANDARD_VERTEX.trim();
            if vertex.trim() != default_vertex {
                shader_json["vertex"] = json!(ShaderJson::encode_to_base64(vertex));
            }
        }

        // Add buffer shaders if they have content (optional)
        for (kind, json_key) in [
            (BufferKind::BufferA, "buffer_a"),
            (BufferKind::BufferB, "buffer_b"),
            (BufferKind::BufferC, "buffer_c"),
            (BufferKind::BufferD, "buffer_d"),
        ] {
            if let Some(buffer) = self.buffers.get(&kind) {
                let (_, fragment) = buffer.get_shaders();
                let trimmed = fragment.trim();

                // Skip empty or comment-only buffers
                if !(trimmed.is_empty() || trimmed.starts_with("//") && trimmed.lines().count() == 1) {
                    shader_json[json_key] = json!(ShaderJson::encode_to_base64(fragment));
                }
            }
        }

        // Add embedded images if loaded (base64-encoded PNG/JPEG)
        for (i, path_opt) in self.image_file_paths.iter().enumerate() {
            if let Some(path) = path_opt {
                // Read image file and encode to base64
                if let Ok(image_bytes) = std::fs::read(path) {
                    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_bytes);
                    let channel_key = format!("ichannel{}", i);
                    shader_json[channel_key] = json!(encoded);
                    log::debug!("Embedded image {} ({} bytes)", i, image_bytes.len());
                }
            }
        }

        // Serialize to pretty JSON
        let json_content = match serde_json::to_string_pretty(&shader_json) {
            Ok(content) => content,
            Err(e) => {
                self.notification_mgr.error(format!("JSON serialization failed: {}", e));
                log::error!("Failed to serialize JSON: {}", e);
                return;
            }
        };

        // Debug: Log exported content structure
        log::debug!("=== EXPORT DEBUG ===");
        log::debug!("Export length: {} bytes", json_content.len());
        log::debug!("Exported keys: {:?}", shader_json.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        log::debug!("===================");

        match std::fs::File::create(&file_path) {
            Ok(mut file) => {
                if file.write_all(json_content.as_bytes()).is_ok() {
                    self.notification_mgr.success("✓ Shader exported to JSON!");
                    log::info!("Shader exported to: {:?}", file_path);
                } else {
                    self.notification_mgr.error("Failed to write file");
                    log::error!("Failed to write to: {:?}", file_path);
                }
            }
            Err(e) => {
                self.notification_mgr.error(format!("Export failed: {}", e));
                log::error!("Failed to create file: {:?}, error: {}", file_path, e);
            }
        }
    }

    fn import_shard(&mut self) {
        // Default to cache/TempRS/shaders/ folder (same as export)
        let cache_shader_dir = dirs::cache_dir()
            .map(|p| p.join("TempRS").join("shaders"))
            .filter(|p| p.exists());

        let mut dialog = rfd::FileDialog::new()
            .add_filter("JSON Shader", &["json"])
            .set_file_name("shader.json");

        if let Some(dir) = cache_shader_dir {
            dialog = dialog.set_directory(dir);
        }

        let file_path = match dialog.pick_file() {
            Some(path) => path,
            None => return,
        };

        // Read the file
        let json_content = match std::fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(e) => {
                self.notification_mgr.error(format!("Failed to read file: {}", e));
                log::error!("Failed to read file {:?}: {}", file_path, e);
                return;
            }
        };

        // Parse and load the JSON shader
        match ShaderJson::from_json(&json_content) {
            Ok(shader_json) => {
                self.load_shader_from_json(shader_json);
                self.notification_mgr.success("✓ Shader imported successfully!");
                log::info!("Shader imported from: {:?}", file_path);
            }
            Err(e) => {
                self.notification_mgr.error(format!("Failed to parse JSON: {}", e));
                log::error!("Failed to parse shader JSON: {}", e);
            }
        }
    }
}
