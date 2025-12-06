//! Notification and Error Handling System
#![allow(dead_code)]
//!
//! Provides user-facing notifications for success, errors, and info messages.
//! Replaces the old "toast" terminology with clearer "notification" naming.

use eframe::egui::{self, Color32, Rect, Vec2};
use std::time::{Duration, Instant};

/// Type of notification to display
#[derive(Clone, Debug)]
pub enum NotificationType {
    /// Success notification (green, auto-dismisses)
    Success,
    /// Error notification (red, sticky by default, shows close button)
    Error,
    /// Info notification (blue, auto-dismisses)
    Info,
    /// Warning notification (yellow, auto-dismisses but longer duration)
    Warning,
}

/// A single notification message
#[derive(Clone, Debug)]
pub struct Notification {
    pub message: String,
    pub notification_type: NotificationType,
    pub created_at: Instant,
    pub duration: Duration,
    pub sticky: bool,      // If true, won't auto-dismiss
    pub dismissed: bool,   // User manually dismissed
}

impl Notification {
    /// Create a new notification
    pub fn new(message: impl Into<String>, notification_type: NotificationType) -> Self {
        let duration = match notification_type {
            NotificationType::Error => Duration::from_secs(10),     // Longer for errors
            NotificationType::Warning => Duration::from_secs(6),    // Medium for warnings
            _ => Duration::from_secs(4),                            // Standard for success/info
        };

        Self {
            message: message.into(),
            notification_type,
            created_at: Instant::now(),
            duration,
            sticky: false,
            dismissed: false,
        }
    }

    /// Create a success notification
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, NotificationType::Success)
    }

    /// Create an error notification (sticky by default)
    pub fn error(message: impl Into<String>) -> Self {
        let mut n = Self::new(message, NotificationType::Error);
        n.sticky = true;  // Errors stay until dismissed
        n
    }

    /// Create an info notification
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, NotificationType::Info)
    }

    /// Create a warning notification
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, NotificationType::Warning)
    }

    /// Check if notification has expired
    pub fn is_expired(&self) -> bool {
        if self.sticky {
            return false;
        }
        self.created_at.elapsed() > self.duration
    }

    /// Get opacity for fade-in/fade-out animation
    pub fn opacity(&self) -> f32 {
        let elapsed = self.created_at.elapsed().as_secs_f32();
        let total = self.duration.as_secs_f32();

        // Fade in over 0.15s
        if elapsed < 0.15 {
            elapsed / 0.15
        }
        // Fade out over last 0.4s
        else if elapsed > total - 0.4 {
            (total - elapsed) / 0.4
        }
        // Fully visible in between
        else {
            1.0
        }
    }
}

/// Manages all active notifications
#[derive(Default)]
pub struct NotificationManager {
    notifications: Vec<Notification>,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Show a notification
    pub fn show(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    /// Show an error notification
    pub fn error(&mut self, message: impl Into<String>) {
        self.show(Notification::error(message));
    }

    /// Show a success notification
    pub fn success(&mut self, message: impl Into<String>) {
        self.show(Notification::success(message));
    }

    /// Show an info notification
    pub fn info(&mut self, message: impl Into<String>) {
        self.show(Notification::info(message));
    }

    /// Show a warning notification
    pub fn warning(&mut self, message: impl Into<String>) {
        self.show(Notification::warning(message));
    }

    /// Check if there are any active notifications
    pub fn has_notifications(&self) -> bool {
        self.notifications.iter().any(|n| !n.is_expired() && !n.dismissed)
    }

    /// Dismiss all error notifications
    pub fn dismiss_errors(&mut self) {
        for n in &mut self.notifications {
            if matches!(n.notification_type, NotificationType::Error) {
                n.dismissed = true;
            }
        }
    }

    /// Dismiss all notifications
    pub fn dismiss_all(&mut self) {
        for n in &mut self.notifications {
            n.dismissed = true;
        }
    }

    /// Render all active notifications
    pub fn render(&mut self, ui: &mut egui::Ui) {
        // Remove expired/dismissed notifications
        self.notifications.retain(|n| !n.is_expired() && !n.dismissed);

        if self.notifications.is_empty() {
            return;
        }

        let screen_rect = ui.ctx().viewport_rect();
        let spacing = 8.0;
        let margin = 20.0;

        // Start from bottom-right corner
        let mut y_offset = screen_rect.max.y - margin;

        for notification in &mut self.notifications {
            // Size based on notification type
            let base_w = match notification.notification_type {
                NotificationType::Error => 700.0,   // Wide for error details
                _ => 280.0,                         // Compact for others
            };

            let base_h = match notification.notification_type {
                NotificationType::Error => {
                    // Dynamic height based on error message length
                    let line_count = notification.message.lines().count().max(1);
                    let min_h = 200.0;
                    let max_h = 600.0;
                    (line_count as f32 * 20.0 + 80.0).clamp(min_h, max_h)
                }
                _ => 50.0,  // Fixed height for others
            };

            // Position from bottom-right
            let x_pos = screen_rect.max.x - base_w - margin;
            y_offset -= base_h;

            let rect = Rect::from_min_size(
                egui::pos2(x_pos, y_offset),
                Vec2::new(base_w, base_h),
            );

            y_offset -= spacing;  // Space for next notification

            // Colors based on type
            let visuals = ui.style().visuals.clone();
            let stroke = visuals.window_stroke();
            let base_fill = visuals.panel_fill;

            let fill = match notification.notification_type {
                NotificationType::Error => Color32::from_rgba_premultiplied(
                    base_fill.r(),
                    base_fill.g().saturating_sub(16),
                    base_fill.b().saturating_sub(16),
                    240,
                ),
                NotificationType::Success => Color32::from_rgba_premultiplied(
                    base_fill.r().saturating_sub(8),
                    base_fill.g().saturating_add(16),
                    base_fill.b().saturating_sub(8),
                    220,
                ),
                NotificationType::Info => Color32::from_rgba_premultiplied(
                    base_fill.r().saturating_sub(8),
                    base_fill.g().saturating_sub(8),
                    base_fill.b().saturating_add(12),
                    220,
                ),
                NotificationType::Warning => Color32::from_rgba_premultiplied(
                    base_fill.r().saturating_add(16),
                    base_fill.g().saturating_add(8),
                    base_fill.b().saturating_sub(16),
                    230,
                ),
            };

            // Apply opacity for fade animation
            let opacity = if notification.sticky {
                1.0
            } else {
                notification.opacity()
            };

            let bg = Color32::from_rgba_premultiplied(
                ((fill.r() as f32) * opacity) as u8,
                ((fill.g() as f32) * opacity) as u8,
                ((fill.b() as f32) * opacity) as u8,
                255,
            );

            // Draw notification background
            ui.painter().rect_filled(rect, 8.0, bg);
            ui.painter().rect_stroke(
                rect,
                8.0,
                egui::Stroke::new(stroke.width, stroke.color),
                egui::StrokeKind::Outside,
            );

            // Render content
            let inner = rect.shrink2(Vec2::new(12.0, 8.0));
            let mut ui_in = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );

            match notification.notification_type {
                NotificationType::Error => {
                    // Full error display with close button and scrolling
                    ui_in.vertical(|ui_col| {
                        ui_col.horizontal(|ui_row| {
                            ui_row.strong("⚠ Error");
                            ui_row.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui_r| {
                                    if ui_r.button("✕").clicked() {
                                        notification.dismissed = true;
                                    }
                                },
                            );
                        });
                        ui_col.add_space(4.0);

                        let text = egui::RichText::new(&notification.message)
                            .family(egui::FontFamily::Monospace)
                            .color(Color32::from_rgb(255, 180, 180))
                            .size(13.0);

                        let max_h = (inner.height() - 40.0).max(100.0);
                        egui::ScrollArea::vertical()
                            .id_salt(format!("notif_scroll_{:?}", notification.created_at))
                            .max_height(max_h)
                            .auto_shrink([false, false])
                            .show(ui_col, |ui_body| {
                                ui_body.add(egui::Label::new(text).wrap().selectable(true));
                            });
                    });
                }
                _ => {
                    // Compact display for success/info/warning
                    ui_in.centered_and_justified(|ui_center| {
                        let (icon, color) = match notification.notification_type {
                            NotificationType::Success => ("✓", Color32::from_rgb(150, 255, 150)),
                            NotificationType::Warning => ("⚠", Color32::from_rgb(255, 220, 100)),
                            NotificationType::Info => ("ℹ", Color32::from_rgb(150, 200, 255)),
                            _ => ("", Color32::WHITE),
                        };

                        let text = egui::RichText::new(format!("{} {}", icon, notification.message))
                            .size(14.0)
                            .color(color);

                        ui_center.label(text);
                    });
                }
            }
        }

        ui.ctx().request_repaint();
    }
}

// Legacy compatibility aliases (intentionally kept for backward compatibility)
#[allow(unused)]
pub use NotificationManager as ToastManager;
#[allow(unused)]
pub use Notification as Toast;
#[allow(unused)]
pub use NotificationType as ToastType;
