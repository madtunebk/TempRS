use eframe::egui_wgpu::wgpu::{Device, Queue, TextureFormat};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use crate::utils::{ShaderPipeline, MultiPassPipelines, ShaderJson};
use sha2::{Sha256, Digest};

const SHADER_CACHE_PATH: &str = ".cache/TempRS/shaders/shader.json";
const SHADER_HOT_RELOAD_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

/// Manages all shader-related state and loading logic
/// Consolidates duplicated shader loading code from player_app.rs
pub struct ShaderManager {
    // Shader pipelines
    pub splash_shader: Option<Arc<ShaderPipeline>>,
    pub multi_pass_shader: Option<Arc<MultiPassPipelines>>,
    pub track_metadata_shader: Option<Arc<ShaderPipeline>>,
    
    // Color correction values (shared across shaders)
    pub gamma: Arc<Mutex<f32>>,
    pub contrast: Arc<Mutex<f32>>,
    pub saturation: Arc<Mutex<f32>>,
    
    // Hot-reload state
    shader_checksum: Option<String>,
    last_hot_reload_check: Instant,
    
    // WGPU resources (cached for shader reinitialization)
    wgpu_device: Option<Arc<Device>>,
    wgpu_queue: Option<Arc<Queue>>,
    wgpu_format: Option<TextureFormat>,
}

impl ShaderManager {
    /// Create new shader manager
    pub fn new() -> Self {
        Self {
            splash_shader: None,
            multi_pass_shader: None,
            track_metadata_shader: None,
            gamma: Arc::new(Mutex::new(1.0)),       // Default: no gamma correction
            contrast: Arc::new(Mutex::new(1.0)),    // Default: normal contrast
            saturation: Arc::new(Mutex::new(1.0)),  // Default: normal saturation
            shader_checksum: None,
            last_hot_reload_check: Instant::now(),
            wgpu_device: None,
            wgpu_queue: None,
            wgpu_format: None,
        }
    }
    
    /// Initialize shaders from WGPU render state
    pub fn initialize(&mut self, render_state: Option<&eframe::egui_wgpu::RenderState>) {
        let Some(render_state) = render_state else {
            log::warn!("[ShaderManager] No WGPU render state available");
            return;
        };
        
        let device = &render_state.device;
        let queue = &render_state.queue;
        let format = render_state.target_format;
        
        // Cache WGPU resources
        self.wgpu_device = Some(Arc::new(device.clone()));
        self.wgpu_queue = Some(Arc::new(queue.clone()));
        self.wgpu_format = Some(format);
        
        // Load splash shader
        self.load_splash_shader(device, format);
        
        // Load track metadata shader
        self.load_track_metadata_shader(device, format);
        
        // Load multi-pass shader from cache or default
        self.load_multipass_shader(device, queue, format);
    }
    
    /// Load splash screen shader (Nebula Drift)
    fn load_splash_shader(&mut self, device: &Device, format: TextureFormat) {
        let splash_wgsl = include_str!("../shaders/splash_bg.wgsl");
        match ShaderPipeline::new(device, format, splash_wgsl) {
            Ok(pipeline) => {
                self.splash_shader = Some(Arc::new(pipeline));
                log::info!("[ShaderManager] Loaded splash shader (Nebula Drift)");
            }
            Err(e) => {
                log::error!("[ShaderManager] Failed to load splash shader: {}", e);
            }
        }
    }
    
    /// Load track metadata background shader
    fn load_track_metadata_shader(&mut self, device: &Device, format: TextureFormat) {
        let metadata_wgsl = include_str!("../shaders/track_metadata_bg.wgsl");
        match ShaderPipeline::new(device, format, metadata_wgsl) {
            Ok(pipeline) => {
                self.track_metadata_shader = Some(Arc::new(pipeline));
                log::info!("[ShaderManager] Loaded track metadata shader");
            }
            Err(e) => {
                log::error!("[ShaderManager] Failed to load track metadata shader: {}", e);
            }
        }
    }
    
    /// Load multi-pass shader from JSON (with cache fallback to embedded default)
    fn load_multipass_shader(&mut self, device: &Device, queue: &Queue, format: TextureFormat) {
        let screen_size = [1920, 1080]; // Default size, will resize on first render
        
        // Try to load from cache
        let json_shader = if let Ok(home_dir) = std::env::var("HOME") {
            let cache_path = format!("{}/{}", home_dir, SHADER_CACHE_PATH);
            if std::path::Path::new(&cache_path).exists() {
                match std::fs::read_to_string(&cache_path) {
                    Ok(json) => {
                        log::info!("[ShaderManager] Loading shader from cache: {}", cache_path);
                        Some(json)
                    }
                    Err(e) => {
                        log::warn!("[ShaderManager] Failed to read cached shader: {}", e);
                        None
                    }
                }
            } else {
                log::info!("[ShaderManager] No cached shader found, using embedded default");
                None
            }
        } else {
            None
        };
        
        // Fallback to embedded default shader
        let json_shader = json_shader.unwrap_or_else(|| {
            include_str!("../assets/shards/demo_multipass.json").to_string()
        });
        
        // Parse and load shader
        self.load_from_json_string(&json_shader, device, queue, format, screen_size);
    }
    
    /// Load shader from JSON string (shared logic for initial load + hot-reload)
    fn load_from_json_string(
        &mut self,
        json_str: &str,
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        screen_size: [u32; 2],
    ) {
        // Compute checksum for hot-reload detection
        let mut hasher = Sha256::new();
        hasher.update(json_str.as_bytes());
        let checksum = format!("{:x}", hasher.finalize());
        
        match ShaderJson::from_json(json_str) {
            Ok(shader_json) => {
                // **COLOR CORRECTION LOADING - SINGLE SOURCE OF TRUTH**
                if let Some(gamma_value) = shader_json.gamma {
                    *self.gamma.lock().unwrap() = gamma_value;
                    log::info!("[ShaderManager] Loaded gamma: {}", gamma_value);
                }
                
                if let Some(contrast_value) = shader_json.contrast {
                    *self.contrast.lock().unwrap() = contrast_value;
                    log::info!("[ShaderManager] Loaded contrast: {}", contrast_value);
                }
                
                if let Some(saturation_value) = shader_json.saturation {
                    *self.saturation.lock().unwrap() = saturation_value;
                    log::info!("[ShaderManager] Loaded saturation: {}", saturation_value);
                }
                
                let multipass_shaders = shader_json.to_shader_map();
                let embedded_images = shader_json.decode_embedded_images();
                let buffer_count = multipass_shaders.len() - 1; // Exclude MainImage
                
                match MultiPassPipelines::new_with_images(device, queue, format, screen_size, &multipass_shaders, &embedded_images) {
                    Ok(pipeline) => {
                        self.multi_pass_shader = Some(Arc::new(pipeline));
                        self.shader_checksum = Some(checksum);
                        
                        if buffer_count > 0 {
                            log::info!("[ShaderManager] Loaded multi-pass shader ({} buffers + MainImage)", buffer_count);
                        } else {
                            log::info!("[ShaderManager] Loaded single-pass shader (MainImage only)");
                        }
                    }
                    Err(e) => {
                        log::error!("[ShaderManager] Failed to create shader pipeline: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("[ShaderManager] Failed to parse shader JSON: {}", e);
            }
        }
    }
    
    /// Check for shader hot-reload (call from update loop)
    pub fn check_hot_reload(&mut self) {
        // Throttle checks to avoid excessive I/O
        if self.last_hot_reload_check.elapsed() < SHADER_HOT_RELOAD_INTERVAL {
            return;
        }
        self.last_hot_reload_check = Instant::now();
        
        // Get cached resources (clone Arcs to avoid borrow issues)
        let device = match &self.wgpu_device {
            Some(d) => Arc::clone(d),
            None => return,
        };
        let queue = match &self.wgpu_queue {
            Some(q) => Arc::clone(q),
            None => return,
        };
        let format = match self.wgpu_format {
            Some(f) => f,
            None => return,
        };
        
        // Check cache path
        let Ok(home_dir) = std::env::var("HOME") else { return };
        let cache_path = format!("{}/{}", home_dir, SHADER_CACHE_PATH);
        
        if !std::path::Path::new(&cache_path).exists() {
            return; // No cached shader to reload
        }
        
        // Read file
        let json_content = match std::fs::read_to_string(&cache_path) {
            Ok(content) => content,
            Err(_) => return, // Silent fail - file might be being written
        };
        
        // Compute new checksum
        let mut hasher = Sha256::new();
        hasher.update(json_content.as_bytes());
        let new_checksum = format!("{:x}", hasher.finalize());
        
        // Check if checksum changed
        if let Some(old_checksum) = &self.shader_checksum {
            if &new_checksum == old_checksum {
                return; // No change
            }
        }
        
        log::info!("[ShaderManager] Detected shader file change, hot-reloading...");
        
        // Reload shader (uses same logic as initial load - NO DUPLICATION!)
        let screen_size = [1920, 1080];
        self.load_from_json_string(&json_content, &device, &queue, format, screen_size);
    }
    
    /// Get gamma reference for shader rendering
    pub fn gamma(&self) -> Arc<Mutex<f32>> {
        Arc::clone(&self.gamma)
    }
    
    /// Get contrast reference for shader rendering
    pub fn contrast(&self) -> Arc<Mutex<f32>> {
        Arc::clone(&self.contrast)
    }
    
    /// Get saturation reference for shader rendering
    pub fn saturation(&self) -> Arc<Mutex<f32>> {
        Arc::clone(&self.saturation)
    }
    
    /// Get multi-pass shader if available
    #[allow(dead_code)]
    pub fn multi_pass(&self) -> Option<Arc<MultiPassPipelines>> {
        self.multi_pass_shader.as_ref().map(Arc::clone)
    }

    /// Get splash shader if available
    pub fn splash(&self) -> Option<Arc<ShaderPipeline>> {
        self.splash_shader.as_ref().map(Arc::clone)
    }

    /// Get track metadata shader if available
    #[allow(dead_code)]
    pub fn track_metadata(&self) -> Option<Arc<ShaderPipeline>> {
        self.track_metadata_shader.as_ref().map(Arc::clone)
    }
}
