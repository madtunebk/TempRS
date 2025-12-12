//#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod api;
mod app;
mod app_state;
mod constants;
mod data;
mod models;
mod screens;
mod services;
mod state;
mod ui_components;
mod utils;

use app::MusicPlayerApp;
use eframe::egui;

// App version and metadata
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "TempRS";
const APP_DESCRIPTION: &str = "SoundCloud Desktop Player";

// SoundCloud OAuth Credentials - loaded from .env file at compile time
// Create .env file from .env.example and add your credentials
// See CREDENTIALS_SETUP.md for instructions on getting credentials
pub const SOUNDCLOUD_CLIENT_ID: &str = env!("SOUNDCLOUD_CLIENT_ID");
pub const SOUNDCLOUD_CLIENT_SECRET: &str = env!("SOUNDCLOUD_CLIENT_SECRET");

const APP_HEIGHT: f32 = 935.0;
const APP_WIDTH: f32 = 1480.0;

fn main() -> Result<(), eframe::Error> {
    // Initialize logger with default settings
    // Set RUST_LOG=debug for verbose output, RUST_LOG=info for normal logs
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .filter_module("wgpu_hal", log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Warn)
        .init();

    log::info!("[Main] Starting {} v{}", APP_NAME, APP_VERSION);

    // Detect GPU and decide rendering backend
    let (use_gpu, gpu_info) = should_use_gpu();
    log::info!(
        "[Main] GPU Detection Result: {} | Using: {}",
        gpu_info,
        if use_gpu { "WGPU (GPU)" } else { "GLOW (CPU)" }
    );

    // Log performance settings based on renderer type
    if use_gpu {
        log::info!("[Main] GPU mode: FPS set to 120, FFT enabled for shaders");
    } else {
        log::info!("[Main] CPU mode: reduced FPS (idle ~4/s, active ~10/s), FFT disabled");
    }

    // Load app icon (music note emoji as fallback)
    let icon_data = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!(
                "{} v{} - {}",
                APP_NAME, APP_VERSION, APP_DESCRIPTION
            ))
            .with_inner_size([APP_WIDTH, APP_HEIGHT]) // Initial size
            .with_min_inner_size([APP_WIDTH, APP_HEIGHT]) // Minimum size
            .with_resizable(true) // Disable resize
            .with_maximize_button(true) // Disable/Enable maximize button
            .with_maximized(true) // Start maximized
            .with_decorations(true) // OS window decorations enabled
            .with_icon(icon_data),
        // Enable vsync in CPU mode to reduce extra swaps
        vsync: !use_gpu,
        persist_window: true, // Remember window position
        renderer: if use_gpu {
            eframe::Renderer::Wgpu // GPU-accelerated with shader support
        } else {
            eframe::Renderer::Glow // CPU rendering, no shader overhead
        },
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            present_mode: eframe::egui_wgpu::wgpu::PresentMode::Fifo,
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        &format!("{} v{}", APP_NAME, APP_VERSION),
        options,
        Box::new(move |cc| {
            // Load emoji font for consistent cross-platform icon rendering
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(MusicPlayerApp::new(cc, use_gpu)))
        }),
    )
}

/// Load app icon - creates a simple colored icon with music note
fn load_icon() -> egui::IconData {
    let (icon_width, icon_height) = (64, 64);
    let mut pixels = vec![0u8; icon_width * icon_height * 4];

    // Create orange gradient background (SoundCloud orange theme)
    for y in 0..icon_height {
        for x in 0..icon_width {
            let idx = (y * icon_width + x) * 4;
            let brightness = 1.0 - (y as f32 / icon_height as f32) * 0.3;

            pixels[idx] = (255.0 * brightness) as u8; // R
            pixels[idx + 1] = (85.0 * brightness) as u8; // G
            pixels[idx + 2] = 0; // B
            pixels[idx + 3] = 255; // A
        }
    }

    // Draw a simple music note in white (center)
    let center_x = icon_width / 2;
    let center_y = icon_height / 2;

    // Vertical stem
    for y in (center_y - 16)..(center_y + 4) {
        for x in (center_x + 4)..(center_x + 8) {
            if x < icon_width && y < icon_height {
                let idx = (y * icon_width + x) * 4;
                pixels[idx] = 255; // R
                pixels[idx + 1] = 255; // G
                pixels[idx + 2] = 255; // B
                pixels[idx + 3] = 255; // A
            }
        }
    }

    // Note head (circle)
    for y in (center_y)..(center_y + 10) {
        for x in (center_x - 6)..(center_x + 4) {
            let dx = x as i32 - center_x as i32;
            let dy = y as i32 - (center_y + 5) as i32;
            if dx * dx + dy * dy < 25 && x < icon_width && y < icon_height {
                let idx = (y * icon_width + x) * 4;
                pixels[idx] = 255; // R
                pixels[idx + 1] = 255; // G
                pixels[idx + 2] = 255; // B
                pixels[idx + 3] = 255; // A
            }
        }
    }

    egui::IconData {
        rgba: pixels,
        width: icon_width as u32,
        height: icon_height as u32,
    }
}

/// Setup custom fonts including emoji support for consistent cross-platform rendering
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Add JetBrains Mono as primary UI font (clean, modern, readable)
    fonts.font_data.insert(
        "JetBrainsMono-Regular".to_string(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );

    fonts.font_data.insert(
        "JetBrainsMono-Medium".to_string(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "assets/fonts/JetBrainsMono-Medium.ttf"
        ))),
    );

    // Add Noto Emoji font (monochrome version - egui doesn't support color emoji)
    fonts.font_data.insert(
        "emoji".to_string(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "assets/fonts/NotoEmoji-Regular.ttf"
        ))),
    );

    // Set JetBrains Mono as primary, emoji as fallback
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "JetBrainsMono-Regular".to_string());

    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .push("emoji".to_string());

    // Monospace uses JetBrains Mono (perfect for code/logs)
    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .insert(0, "JetBrainsMono-Medium".to_string());

    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .push("emoji".to_string());

    ctx.set_fonts(fonts);
    log::info!("[Main] Custom fonts loaded: JetBrains Mono + Noto Emoji");
}

/// Detect best GPU available and decide whether to use GPU or CPU rendering
/// Returns (use_gpu: bool, gpu_info: String)
fn should_use_gpu() -> (bool, String) {
    use eframe::egui_wgpu::wgpu;

    // Try to create wgpu instance to enumerate adapters
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapters = instance.enumerate_adapters(wgpu::Backends::all());
    let adapter_count = adapters.len();

    log::info!("[GPU] Scanning {} adapter(s)...", adapter_count);

    if adapter_count == 0 {
        log::warn!("[GPU] No GPU found - using CPU rendering");
        return (false, "No GPU found".to_string());
    }

    let mut has_discrete = false;
    let mut has_integrated = false;
    let mut discrete_name = String::new();
    let mut integrated_name = String::new();

    // Scan all adapters
    for adapter in &adapters {
        let info = adapter.get_info();
        log::info!(
            "[GPU] Found: {} | Type: {:?} | Backend: {:?}",
            info.name,
            info.device_type,
            info.backend
        );

        match info.device_type {
            wgpu::DeviceType::DiscreteGpu => {
                has_discrete = true;
                discrete_name = info.name.clone();
                log::info!("[GPU] ✓ Dedicated GPU detected: {}", info.name);
            }
            wgpu::DeviceType::IntegratedGpu => {
                has_integrated = true;
                integrated_name = info.name.clone();
                log::info!("[GPU] ⚠ Integrated GPU detected: {}", info.name);
            }
            wgpu::DeviceType::VirtualGpu => {
                log::warn!("[GPU] Virtual GPU detected (VM?): {}", info.name);
            }
            wgpu::DeviceType::Cpu => {
                log::warn!("[GPU] Software renderer detected: {}", info.name);
            }
            _ => {}
        }
    }

    // Decision logic: Discrete > CPU (avoid iGPU burning CPU)
    if has_discrete {
        log::info!("[GPU] Using HighPerformance mode (dedicated GPU)");
        (true, format!("{} (Dedicated)", discrete_name))
    } else if has_integrated {
        log::warn!("[GPU] Only integrated GPU available");
        log::warn!("[GPU] Using CPU rendering to prevent high CPU usage from iGPU");
        log::warn!("[GPU] Shaders will be disabled");
        (
            false,
            format!("{} (iGPU - disabled to save CPU)", integrated_name),
        )
    } else {
        log::warn!("[GPU] No dedicated/integrated GPU found - using CPU rendering");
        (false, "No suitable GPU".to_string())
    }
}

#[allow(dead_code)]
fn detect_window_size() -> (f32, f32) {
    if let Some(monitor) = eframe::egui::Context::default().input(|i| i.viewport().monitor_size) {
        // Use 1920x1080 if monitor is large enough, otherwise 1280x720
        if monitor.x >= 1920.0 && monitor.y >= 1080.0 {
            (1920.0, 1080.0)
        } else {
            (1280.0, 720.0)
        }
    } else {
        // Default to 1280x720 if detection fails
        (1280.0, 720.0)
    }
}
