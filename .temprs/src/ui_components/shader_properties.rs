use eframe::egui;
use std::sync::{Arc, Mutex};

/// Actions that can be triggered from the Shader Properties window
pub enum ShaderPropertiesAction {
    LoadPreset(String),
    LoadAudioFile(String),
    LoadImageFile(usize, String), // (channel_index, file_path)
    ExportShard,
    ImportShard,
    None,
}

/// Render the Shader Properties window
#[allow(clippy::too_many_arguments)]
pub fn render(
    ctx: &egui::Context,
    show_window: &mut bool,
    audio_file_path: &Option<String>,
    image_file_paths: &[Option<String>; 4],
    selected_channel: &mut usize,
    debug_audio: &mut bool,
    debug_bass: &mut f32,
    debug_mid: &mut f32,
    debug_high: &mut f32,
    bass_energy: &Arc<Mutex<f32>>,
    mid_energy: &Arc<Mutex<f32>>,
    high_energy: &Arc<Mutex<f32>>,
) -> ShaderPropertiesAction {
    let mut action = ShaderPropertiesAction::None;
    let mut close_requested = false;

    egui::Window::new("🎨 Shader Properties")
        .id(egui::Id::new("shader_properties_window"))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .default_size([420.0, 600.0])
        .open(show_window)
        .show(ctx, |ui| {
            ui.set_min_width(400.0);
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

            // Presets Section with styled frame
            ui.push_id("shader_presets_section", |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(25, 25, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 70)))
                .corner_radius(6.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Shader Presets").size(16.0).strong());
                    ui.add_space(8.0);

                    let button_size = egui::vec2(ui.available_width(), 32.0);

                    if ui.add_sized(button_size, egui::Button::new(
                        egui::RichText::new("Default (Audio Visualizer)").size(13.0)
                    )).clicked() {
                        action = ShaderPropertiesAction::LoadPreset("default".to_string());
                    }

                    if ui.add_sized(button_size, egui::Button::new(
                        egui::RichText::new("Psychedelic Spiral").size(13.0)
                    )).clicked() {
                        action = ShaderPropertiesAction::LoadPreset("psychedelic".to_string());
                    }

                    if ui.add_sized(button_size, egui::Button::new(
                        egui::RichText::new("Infinite Tunnel").size(13.0)
                    )).clicked() {
                        action = ShaderPropertiesAction::LoadPreset("tunnel".to_string());
                    }

                    if ui.add_sized(button_size, egui::Button::new(
                        egui::RichText::new("Raymarched Boxes").size(13.0)
                    )).clicked() {
                        action = ShaderPropertiesAction::LoadPreset("raymarch".to_string());
                    }

                    if ui.add_sized(button_size, egui::Button::new(
                        egui::RichText::new("Julia Set Fractal").size(13.0)
                    )).clicked() {
                        action = ShaderPropertiesAction::LoadPreset("fractal".to_string());
                    }

                    if ui.add_sized(button_size, egui::Button::new(
                        egui::RichText::new("Image Demo").size(13.0)
                    )).clicked() {
                        action = ShaderPropertiesAction::LoadPreset("image_demo".to_string());
                    }
                });
            });

            ui.add_space(12.0);

            // Audio Section with styled frame
            ui.push_id("audio_section", |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(25, 25, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 70)))
                .corner_radius(6.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Audio").size(16.0).strong());
                    ui.add_space(8.0);

                    // Audio file display
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("File:").strong().size(12.0));
                        ui.add_space(4.0);

                        let file_text = if let Some(path) = audio_file_path {
                            std::path::Path::new(path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string()
                        } else {
                            "No audio loaded".to_string()
                        };

                        ui.label(
                            egui::RichText::new(file_text)
                                .monospace()
                                .size(11.0)
                                .color(if audio_file_path.is_some() {
                                    egui::Color32::from_rgb(120, 220, 120)
                                } else {
                                    egui::Color32::from_rgb(140, 140, 150)
                                })
                        );
                    });

                    ui.add_space(8.0);

                    if ui.add_sized([ui.available_width(), 30.0], egui::Button::new(
                        egui::RichText::new("Load Audio File...").size(13.0)
                    )).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Audio", &["mp3", "wav", "ogg", "flac"])
                            .pick_file()
                        {
                            action = ShaderPropertiesAction::LoadAudioFile(path.to_string_lossy().to_string());
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.checkbox(debug_audio,
                        egui::RichText::new("Debug Mode (Manual Control)").size(12.0));

                    ui.add_space(8.0);

                    if *debug_audio {
                        ui.label(egui::RichText::new("Manual Controls:").strong().size(12.0));
                        ui.add_space(4.0);
                        ui.add(egui::Slider::new(debug_bass, 0.0..=1.0)
                            .text("Bass").show_value(true));
                        ui.add(egui::Slider::new(debug_mid, 0.0..=1.0)
                            .text("Mid").show_value(true));
                        ui.add(egui::Slider::new(debug_high, 0.0..=1.0)
                            .text("High").show_value(true));
                    } else {
                        ui.label(egui::RichText::new("Live Audio Levels:").strong().size(12.0));
                        ui.add_space(4.0);

                        let bass = *bass_energy.lock().unwrap();
                        let mid = *mid_energy.lock().unwrap();
                        let high = *high_energy.lock().unwrap();

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Bass").size(12.0).strong());
                            ui.add(egui::ProgressBar::new(bass)
                                .text(format!("{:.2}", bass))
                                .desired_width(ui.available_width()));
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Mid").size(12.0).strong());
                            ui.add(egui::ProgressBar::new(mid)
                                .text(format!("{:.2}", mid))
                                .desired_width(ui.available_width()));
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("High").size(12.0).strong());
                            ui.add(egui::ProgressBar::new(high)
                                .text(format!("High {:.2}", high))
                                .desired_width(ui.available_width()));
                            });
                        }
                    });
            });

            ui.add_space(12.0);

            // Image Section with styled frame
            ui.push_id("image_section", |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(25, 25, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 70)))
                .corner_radius(6.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Image Textures").size(16.0).strong());
                    ui.add_space(8.0);

                    // Channel selector and status in compact layout
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Target:").strong().size(12.0));
                        egui::ComboBox::from_id_salt("image_channel_selector")
                            .width(90.0)
                            .selected_text(format!("iChannel{}", selected_channel))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(selected_channel, 0, "iChannel0");
                                ui.selectable_value(selected_channel, 1, "iChannel1");
                                ui.selectable_value(selected_channel, 2, "iChannel2");
                                ui.selectable_value(selected_channel, 3, "iChannel3");
                            });
                        
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);
                        
                        // Show compact status: "0:✓ 1:- 2:✓ 3:-"
                        ui.label(egui::RichText::new("Status:").strong().size(12.0));
                        ui.add_space(4.0);
                        for (i, image_path) in image_file_paths.iter().enumerate() {
                            let status = if image_path.is_some() { "✓" } else { "—" };
                            ui.label(
                                egui::RichText::new(format!("{}:{}", i, status))
                                    .monospace()
                                    .size(11.0)
                                    .color(if image_path.is_some() {
                                        egui::Color32::from_rgb(120, 220, 120)
                                    } else {
                                        egui::Color32::from_rgb(100, 100, 110)
                                    })
                            );
                        }
                    });

                    ui.add_space(8.0);

                    if ui.add_sized([ui.available_width(), 30.0], egui::Button::new(
                        egui::RichText::new(format!("Load to iChannel{}...", selected_channel)).size(13.0)
                    )).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "gif", "webp"])
                            .pick_file()
                        {
                            action = ShaderPropertiesAction::LoadImageFile(*selected_channel, path.to_string_lossy().to_string());
                        }
                    }
                });
            });

            ui.add_space(12.0);            // Import/Export Section with styled frame
            ui.push_id("import_export_section", |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(25, 25, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 70)))
                .corner_radius(6.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Import/Export").size(16.0).strong());
                    ui.add_space(8.0);

                    if ui.add_sized([ui.available_width(), 32.0], egui::Button::new(
                        egui::RichText::new("Import Shard...").size(13.0)
                    )).clicked() {
                        action = ShaderPropertiesAction::ImportShard;
                    }

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Load shader from JSON file (Ctrl+I)")
                            .size(10.0)
                            .color(egui::Color32::from_rgb(140, 140, 150))
                    );

                    ui.add_space(8.0);

                    if ui.add_sized([ui.available_width(), 32.0], egui::Button::new(
                        egui::RichText::new("Export Shard...").size(13.0)
                    )).clicked() {
                        action = ShaderPropertiesAction::ExportShard;
                    }

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Save all buffers to JSON file (Ctrl+E)")
                            .size(10.0)
                            .color(egui::Color32::from_rgb(140, 140, 150))
                    );
                });
            });

            ui.add_space(15.0);

            ui.vertical_centered(|ui| {
                if ui.add_sized([120.0, 36.0], egui::Button::new(
                    egui::RichText::new("Close").size(15.0).strong()
                )).clicked() {
                    close_requested = true;
                }
            });
        });

    if close_requested {
        *show_window = false;
    }

    action
}
