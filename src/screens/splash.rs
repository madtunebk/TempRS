use eframe::egui;
use crate::app::player_app::MusicPlayerApp;
use crate::utils::shader::ShaderCallback;

pub fn render_splash_screen(app: &mut MusicPlayerApp, ctx: &egui::Context) {
    // Request repaint with 60 FPS limit to reduce CPU usage
    ctx.request_repaint_after(std::time::Duration::from_millis(16));
    
    // Add animated shader background to BACKGROUND painter (renders first, behind everything)
    if let Some(shader) = &app.splash_shader {
        let rect = ctx.content_rect();
        
        let callback = egui_wgpu::Callback::new_paint_callback(
            rect,
            ShaderCallback {
                shader: shader.clone(),
                audio_bass: 0.0,   // No audio on splash screen
                audio_mid: 0.0,
                audio_high: 0.0,
            },
        );
        
        // Use background painter instead of debug_painter so UI renders on top
        ctx.layer_painter(egui::LayerId::background()).add(callback);
    } else {
        log::error!("[Shader] Pipeline not initialized!");
    }
    
    // Load logo texture if not already loaded
    if app.logo_texture.is_none() {
        app.logo_texture = Some(load_logo_texture(ctx));
    }
    
    // Load no_artwork texture if not already loaded
    if app.no_artwork_texture.is_none() {
        app.no_artwork_texture = Some(load_no_artwork_texture(ctx));
    }
    
    // UI Panel with full-screen semi-transparent black overlay
    // SHADER VISIBILITY: Adjust alpha value (0-255) to control shader visibility
    // Lower value = more shader visible, Higher value = darker overlay
    // Current: 180 (70% opacity) - try values between 150-230
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180)))
        .show(ctx, |ui| {
            // Static alpha - no animation for better performance
            let alpha = 255u8;
            
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.add_space(20.0);
                
                // Logo image
                if let Some(logo_texture) = &app.logo_texture {
                    render_logo_image(ui, logo_texture);
                }
                
                ui.add_space(40.0);
                
                // Show different states based on what's happening
                if app.is_authenticating {
                    // User clicked login - waiting for browser authentication
                    render_authenticating_status(ui, alpha);
                    ui.add_space(10.0);
                    render_loading_text(ui, alpha, "Check your browser to complete login...");
                } else if app.token_check_done {
                    // Token check completed - check if we have a valid token
                    let has_valid_token = if let Some(oauth_manager) = &app.oauth_manager {
                        oauth_manager.get_token().is_some()
                    } else {
                        false
                    };
                    
                    if has_valid_token {
                        // Valid token found - waiting for minimum splash duration
                        render_loading_spinner(ui, alpha);
                        ui.add_space(10.0);
                        render_loading_text(ui, alpha, "Loading your music library...");
                    } else {
                        // No valid token - show login button
                        if render_login_button(ui, alpha) {
                            log::info!("Login button clicked");
                            start_oauth_flow(app);
                        }
                        
                        ui.add_space(20.0);
                        render_loading_text(ui, alpha, "A desktop player for SoundCloud");
                    }
                } else {
                    // Initial token check in progress
                    render_loading_spinner(ui, alpha);
                    ui.add_space(10.0);
                    render_loading_text(ui, alpha, "Checking authentication...");
                }
            });
        });
}
fn start_oauth_flow(app: &mut MusicPlayerApp) {
    if let Some(oauth_manager) = &app.oauth_manager {
        app.is_authenticating = true;
        app.login_message_shown = false; // Reset for next time
        app.token_check_done = false; // Reset to allow re-check after auth
        
        // Generate state for CSRF protection
        let state = format!("{}", rand::random::<u64>());
        
        // Get authorization URL
        let auth_url = oauth_manager.get_authorization_url(&state);
        
        // Open browser for authentication
        if let Err(e) = webbrowser::open(&auth_url) {
            log::error!("Failed to open browser: {}", e);
            app.is_authenticating = false;
            return;
        }
        
        // Spawn async task to handle OAuth callback
        let oauth_manager = app.oauth_manager.clone();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                if let Some(manager) = oauth_manager {
                    match manager.start_oauth_callback_server().await {
                        Ok(code) => {
                            log::info!("Received authorization code: {}", code);
                            match manager.exchange_code_for_token(&code).await {
                                Ok(token) => {
                                    log::info!("Successfully authenticated! Token expires at: {}", token.expires_at);
                                    // Note: App state will be updated in splash render when token is detected
                                }
                                Err(e) => {
                                    log::error!("Failed to exchange code for token: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("OAuth callback failed: {}", e);
                        }
                    }
                }
            });
        });
    }
}

fn load_logo_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let logo_bytes = include_bytes!("../assets/logo.png");
    let mut image = image::load_from_memory(logo_bytes)
        .expect("Failed to load logo")
        .to_rgba8();
    
    // Remove black background - make dark pixels transparent
    for pixel in image.pixels_mut() {
        let (r, g, b) = (pixel[0], pixel[1], pixel[2]);
        
        // If pixel is very dark (near black), make it transparent
        if r < 30 && g < 30 && b < 30 {
            pixel[3] = 0; // Set alpha to 0 (transparent)
        }
    }
    
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.as_flat_samples();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
    
    ctx.load_texture("logo", color_image, egui::TextureOptions::LINEAR)
}

fn load_no_artwork_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let artwork_bytes = include_bytes!("../assets/no_artwork.png");
    let image = image::load_from_memory(artwork_bytes)
        .expect("Failed to load no_artwork.png")
        .to_rgba8();
    
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.as_flat_samples();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
    
    ctx.load_texture("no_artwork", color_image, egui::TextureOptions::LINEAR)
}

fn render_logo_image(ui: &mut egui::Ui, texture: &egui::TextureHandle) {
    // LOGO SIZE: Reduced 25% for better balance (400 → 300)
    // Try: 280x280 (smaller), 320x320 (larger)
    let img_size = egui::vec2(300.0, 300.0);
    
    ui.vertical_centered(|ui| {
        // Get rect for logo
        let (rect, _) = ui.allocate_exact_size(img_size, egui::Sense::hover());
        /*
        // CIRCLE SIZE: Extra padding around logo (current: +20px)
        // Try: +15 (tighter), +25 (more space), +30 (loose)
        let circle_radius = (img_size.x / 2.0) + 20.0;
        let circle_center = rect.center();
        
        // CIRCLE BACKGROUND: Black sticker background (static, no animation)
        // Color: (R, G, B, Alpha) - try (10,10,15) for dark blue-black
        // Alpha: 220 (86%)
        ui.painter().circle_filled(
            circle_center,
            circle_radius,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 220)
        );
        
        // INNER GLOW: Subtle 2-3px glow inside the circle for premium look
        // Multiple thin strokes create soft inner glow effect
        // Stroke width: 2.5px (try 2.0-3.5)
        // Color: Soft orange glow with medium transparency

        ui.painter().circle_stroke(
            circle_center,
            circle_radius - 1.0,
            egui::Stroke::new(2.5, egui::Color32::from_rgba_unmultiplied(255, 140, 60, 80))
        );
        */
        // Draw logo on top (static, full opacity)
        ui.painter().image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE
        );
    });
}

fn render_login_button(ui: &mut egui::Ui, alpha: u8) -> bool {
    // BUTTON STYLING SETTINGS
    ui.add(
        egui::Button::new(
            egui::RichText::new("Login to SoundCloud")
                .size(20.0)  // TEXT SIZE: Try 18.0-24.0
                .color(egui::Color32::WHITE)  // TEXT COLOR: White for contrast
                .strong()  // BOLD TEXT: Remove .strong() for normal weight
        )
        .fill(egui::Color32::from_rgba_unmultiplied(255, 85, 0, alpha))  // BUTTON COLOR: SoundCloud orange (255,85,0)
        .corner_radius(30.0)  // ROUNDNESS: Try 20.0 (less round), 40.0 (more round)
        .min_size(egui::vec2(280.0, 60.0))  // BUTTON SIZE: (width, height) - try (250,55) or (300,65)
    ).clicked()
}

fn render_loading_text(ui: &mut egui::Ui, alpha: u8, text: &str) {
    // TEXT STYLING SETTINGS
    ui.vertical_centered(|ui| {
        ui.add(
            egui::Label::new(
                egui::RichText::new(text)
                    .size(17.0)  // TEXT SIZE: Try 15.0-20.0
                    .family(egui::FontFamily::Proportional)  // FONT: Proportional (default) or Monospace
                    .color(egui::Color32::from_rgba_unmultiplied(230, 230, 230, alpha))  // TEXT COLOR: Light gray (230,230,230)
            )
        );
    });
}

fn render_authenticating_status(ui: &mut egui::Ui, alpha: u8) {
    // SPINNER SETTINGS
    ui.add(
        egui::Spinner::new()
            .size(40.0)  // SPINNER SIZE: Try 30.0-50.0
            .color(egui::Color32::from_rgba_unmultiplied(255, 120, 40, alpha))  // SPINNER COLOR: Orange glow (255,120,40)
    );
}

fn render_loading_spinner(ui: &mut egui::Ui, alpha: u8) {
    // Same as authenticating spinner
    render_authenticating_status(ui, alpha);
}
