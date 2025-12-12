/// Centralized color palette for TempRS application
/// Following dark theme design with consistent color hierarchy
use eframe::egui::Color32;

// Primary brand colors
#[allow(dead_code)]
pub const ORANGE: Color32 = Color32::from_rgb(255, 85, 0);
#[allow(dead_code)]
pub const ORANGE_HOVER: Color32 = Color32::from_rgb(255, 138, 43);
#[allow(dead_code)]
pub const ORANGE_LIGHT: Color32 = Color32::from_rgb(255, 120, 40);

// Background colors (from darkest to lightest)
pub const BG_MAIN: Color32 = Color32::from_rgb(16, 16, 16); // #101010 - Main background
pub const BG_CARD: Color32 = Color32::from_rgb(18, 18, 18); // #121212 - Cards, panels
pub const BG_HOVER: Color32 = Color32::from_rgb(26, 26, 26); // #1A1A1A - Hover states
#[allow(dead_code)]
pub const BG_INPUT: Color32 = Color32::from_rgb(30, 30, 32); // Input fields
pub const BG_BUTTON: Color32 = Color32::from_rgb(35, 35, 40); // Default buttons
pub const BG_BUTTON_HOVER: Color32 = Color32::from_rgb(45, 45, 50); // Button hover

// Legacy color names for backward compatibility (will eventually be replaced)
pub const DARK_GRAY: Color32 = Color32::from_rgb(30, 30, 32); // Maps to BG_INPUT
pub const MID_GRAY: Color32 = Color32::from_rgb(45, 45, 50); // Maps to BG_BUTTON_HOVER
pub const LIGHT_GRAY: Color32 = Color32::from_rgb(160, 160, 160); // Maps to TEXT_SECONDARY

// Text colors
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 240); // Primary text
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 160); // Secondary text
pub const TEXT_TERTIARY: Color32 = Color32::from_rgb(120, 120, 120); // Tertiary/disabled text
#[allow(dead_code)]
pub const TEXT_INVERSE: Color32 = Color32::WHITE; // Text on colored backgrounds

// Skeleton/loading colors
#[allow(dead_code)]
pub const SKELETON_BASE: Color32 = Color32::from_rgb(55, 55, 60);
#[allow(dead_code)]
pub const SKELETON_PULSE: Color32 = Color32::from_rgb(60, 60, 65);

// Border colors
#[allow(dead_code)]
pub const BORDER_DEFAULT: Color32 = Color32::from_rgb(40, 40, 45);
#[allow(dead_code)]
pub const BORDER_HOVER: Color32 = ORANGE;
#[allow(dead_code)]
pub const BORDER_FOCUS: Color32 = Color32::from_rgb(60, 60, 65);

// Special states
#[allow(dead_code)]
pub const SUCCESS: Color32 = Color32::from_rgb(76, 175, 80);
pub const ERROR: Color32 = Color32::from_rgb(255, 100, 100);
#[allow(dead_code)]
pub const WARNING: Color32 = Color32::from_rgb(255, 193, 7);

// Overlay colors
pub const OVERLAY_DARK: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 150);
#[allow(dead_code)]
pub const OVERLAY_LIGHT: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 100);
#[allow(dead_code)]
pub const OVERLAY_BADGE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 180);
