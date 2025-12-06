use eframe::{egui, NativeOptions};

mod funcs;
mod screens;
mod ui_components;
mod utils;

fn main() {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Set up panic hook to log panics instead of just crashing
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        
        let location = if let Some(loc) = panic_info.location() {
            format!(" at {}:{}:{}", loc.file(), loc.line(), loc.column())
        } else {
            String::new()
        };
        
        log::error!("Panic occurred{}: {}", location, message);
        eprintln!("PANIC{}: {}", location, message);
    }));

    log::info!("Application starting...");

    let mut native_options = NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            // Use a custom error handler that logs instead of panicking
            on_surface_error: std::sync::Arc::new(|err| {
                log::error!("WGPU surface error: {}", err);
                eframe::egui_wgpu::SurfaceErrorAction::SkipFrame
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    // Auto-detect monitor size and position window, or fallback to defaults
    let (window_size, window_pos) = if let Some((x, y, w, h)) = utils::detect_primary_monitor_xrandr() {
        let ww = (w as f32 * 0.75).round();
        let hh = (h as f32 * 0.75).round();
        let size = egui::vec2(ww, hh);
        let px = x + ((w - ww as i32) / 2);
        let py = y + ((h - hh as i32) / 2);
        let pos = Some(egui::Pos2::new(px as f32, py as f32));
        (size, pos)
    } else {
        // Fallback: 75% of 1920x1080
        (egui::vec2(1440.0, 810.0), None)
    };

    let mut vp = egui::ViewportBuilder::default().with_inner_size([window_size.x, window_size.y]);
    if let Some(pos) = window_pos {
        vp = vp.with_position([pos.x, pos.y]);
    }
    native_options.viewport = vp;

    let result = eframe::run_native(
        "WebShard Editor",
        native_options,
        Box::new(|cc| {
            // Register fonts and configure styles
            utils::register_error_fonts(&cc.egui_ctx);
            utils::apply_editor_theme(&cc.egui_ctx);
            Ok(Box::new(screens::editor::TopApp::new(cc)))
        }),
    );

    if let Err(e) = result {
        log::error!("Application error: {}", e);
    } else {
        log::info!("Application terminated normally");
    }
}
