#![allow(dead_code)]
use eframe::egui::{self, Color32, Rect, Vec2};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ToastType {
    Success,
    Error,
    Info,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub message: String,
    pub toast_type: ToastType,
    pub created_at: Instant,
    pub duration: Duration,
    pub sticky: bool,
    pub dismissed: bool,
}

impl Toast {
    pub fn new(message: impl Into<String>, toast_type: ToastType) -> Self {
        Self {
            message: message.into(),
            toast_type,
            created_at: Instant::now(),
            duration: Duration::from_secs(4),
            sticky: false,
            dismissed: false,
        }
    }

    #[allow(dead_code)]
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Success)
    }
    #[allow(dead_code)]
    pub fn error(message: impl Into<String>) -> Self {
        let mut t = Self::new(message, ToastType::Error);
        t.sticky = true;
        t
    }
    #[allow(dead_code)]
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Info)
    }

    pub fn is_expired(&self) -> bool {
        if self.sticky {
            return false;
        }
        self.created_at.elapsed() > self.duration
    }
    pub fn opacity(&self) -> f32 {
        let elapsed = self.created_at.elapsed().as_secs_f32();
        let total = self.duration.as_secs_f32();
        if elapsed < 0.15 {
            elapsed / 0.15
        } else if elapsed > total - 0.4 {
            (total - elapsed) / 0.4
        } else {
            1.0
        }
    }
}

#[derive(Default)]
pub struct ToastManager {
    pub toasts: Vec<Toast>,
}

impl ToastManager {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn show(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }
    #[allow(dead_code)]
    pub fn show_error(&mut self, message: impl Into<String>) {
        self.show(Toast::error(message));
    }
    pub fn show_success(&mut self, message: impl Into<String>) {
        self.show(Toast::success(message));
    }
    #[allow(dead_code)]
    pub fn show_info(&mut self, message: impl Into<String>) {
        self.show(Toast::info(message));
    }
    pub fn has_toasts(&self) -> bool {
        self.toasts.iter().any(|t| !t.is_expired() && !t.dismissed)
    }
    #[allow(dead_code)]
    pub fn dismiss_errors(&mut self) {
        for t in &mut self.toasts {
            if let ToastType::Error = t.toast_type {
                t.dismissed = true;
            }
        }
    }
    
    pub fn dismiss_all(&mut self) {
        for t in &mut self.toasts {
            t.dismissed = true;
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        self.toasts.retain(|t| !t.is_expired() && !t.dismissed);
        if self.toasts.is_empty() {
            return;
        }
        let screen_rect = ui.ctx().viewport_rect();
        let spacing = 8.0;
        let margin = 20.0;
        
        // Start from bottom-right corner
        let mut y_offset = screen_rect.max.y - margin;
        
        for toast in &mut self.toasts {
            // Size presets
            let base_w = match toast.toast_type {
                ToastType::Error => 700.0,
                _ => 280.0,  // Smaller for success/info toasts
            };
            
            let base_h = match toast.toast_type {
                ToastType::Error => {
                    let line_count = toast.message.lines().count().max(1);
                    let min_h = 200.0;
                    let max_h = 600.0;
                    (line_count as f32 * 20.0 + 80.0).clamp(min_h, max_h)
                }
                _ => 50.0,  // Compact for success/info
            };
            
            // Position from bottom-right
            let x_pos = screen_rect.max.x - base_w - margin;
            y_offset -= base_h;
            
            let rect = Rect::from_min_size(
                egui::pos2(x_pos, y_offset),
                Vec2::new(base_w, base_h)
            );
            
            y_offset -= spacing;  // Space for next toast

            // Theme-aware colors
            let visuals = ui.style().visuals.clone();
            let stroke = visuals.window_stroke();
            let base_fill = visuals.panel_fill;
            let fill = match toast.toast_type {
                ToastType::Error => Color32::from_rgba_premultiplied(
                    base_fill.r(),
                    base_fill.g().saturating_sub(16),
                    base_fill.b().saturating_sub(16),
                    240,
                ),
                ToastType::Success => Color32::from_rgba_premultiplied(
                    base_fill.r().saturating_sub(8),
                    base_fill.g().saturating_add(16),
                    base_fill.b().saturating_sub(8),
                    220,
                ),
                ToastType::Info => Color32::from_rgba_premultiplied(
                    base_fill.r().saturating_sub(8),
                    base_fill.g().saturating_sub(8),
                    base_fill.b().saturating_add(12),
                    220,
                ),
            };

            let opacity = if toast.sticky { 1.0 } else { toast.opacity() };
            let bg = Color32::from_rgba_premultiplied(
                ((fill.r() as f32) * opacity) as u8,
                ((fill.g() as f32) * opacity) as u8,
                ((fill.b() as f32) * opacity) as u8,
                255,
            );
            ui.painter().rect_filled(rect, 8.0, bg);
            ui.painter().rect_stroke(
                rect,
                8.0,
                egui::Stroke::new(stroke.width, stroke.color),
                egui::StrokeKind::Outside,
            );

            // Use layout to render messages
            let inner = rect.shrink2(Vec2::new(12.0, 8.0));
            let mut ui_in = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            
            match toast.toast_type {
                ToastType::Error => {
                    // Full error display with close button and scroll
                    ui_in.vertical(|ui_col| {
                        ui_col.horizontal(|ui_row| {
                            ui_row.strong("⚠ WGSL Compilation Error");
                            ui_row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui_r| {
                                if ui_r.button("✕").clicked() {
                                    toast.dismissed = true;
                                }
                            });
                        });
                        ui_col.add_space(4.0);
                        
                        let text = egui::RichText::new(&toast.message)
                            .family(egui::FontFamily::Monospace)
                            .color(Color32::from_rgb(255, 180, 180))
                            .size(13.0);
                        let max_h = (inner.height() - 40.0).max(100.0);
                        egui::ScrollArea::vertical()
                            .id_salt(format!("toast_scroll_{:?}", toast.created_at))
                            .max_height(max_h)
                            .auto_shrink([false, false])
                            .show(ui_col, |ui_body| {
                                ui_body.add(egui::Label::new(text).wrap().selectable(true));
                            });
                    });
                }
                _ => {
                    // Compact success/info display
                    ui_in.centered_and_justified(|ui_center| {
                        let text = egui::RichText::new(&toast.message)
                            .size(14.0)
                            .color(Color32::WHITE);
                        ui_center.label(text);
                    });
                }
            }
        }
        ui.ctx().request_repaint();
    }
}
