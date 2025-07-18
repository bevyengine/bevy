//! # Editor Theme System
//!
//! This module provides a comprehensive theming system for the Bevy Editor,
//! including color schemes, typography, and layout constants for consistent
//! styling across all editor components.
//!
//! ## Available Themes
//!
//! - **DarkTheme**: Professional dark color scheme optimized for long editing sessions
//! - **EditorTheme**: Configurable theme trait for custom color schemes
//!
//! ## Color Categories
//!
//! The theme system organizes colors into logical categories:
//! - **Background**: Primary, secondary, tertiary, and header backgrounds
//! - **Border**: Various border colors for different UI elements
//! - **Text**: Text colors for different contexts and states
//! - **Interactive**: Button and selection states
//! - **Status**: Success, warning, and error indicators
//!
//! ## Usage
//!
//! Colors can be accessed as constants from the theme structs:
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use bevy_editor::themes::DarkTheme;
//!
//! fn setup(mut commands: Commands) {
//!     // Use theme colors in UI components
//!     commands.spawn((
//!         Node::default(),
//!         BackgroundColor(DarkTheme::BACKGROUND_PRIMARY),
//!     ));
//! }
//! ```

use bevy::prelude::*;

/// Dark theme color scheme for the editor
pub struct DarkTheme;

impl DarkTheme {
    // Background colors
    pub const BACKGROUND_PRIMARY: Color = Color::srgb(0.16, 0.16, 0.16);
    pub const BACKGROUND_SECONDARY: Color = Color::srgb(0.18, 0.18, 0.18);
    pub const BACKGROUND_TERTIARY: Color = Color::srgb(0.14, 0.14, 0.14);
    pub const BACKGROUND_HEADER: Color = Color::srgb(0.22, 0.22, 0.22);
    
    // Border colors
    pub const BORDER_PRIMARY: Color = Color::srgb(0.35, 0.35, 0.35);
    pub const BORDER_SECONDARY: Color = Color::srgb(0.4, 0.4, 0.4);
    pub const BORDER_ACCENT: Color = Color::srgb(0.45, 0.45, 0.45);
    
    // Text colors
    pub const TEXT_PRIMARY: Color = Color::srgb(0.95, 0.95, 0.95);
    pub const TEXT_SECONDARY: Color = Color::srgb(0.9, 0.9, 0.9);
    pub const TEXT_MUTED: Color = Color::srgb(0.65, 0.65, 0.65);
    pub const TEXT_DISABLED: Color = Color::srgb(0.6, 0.6, 0.6);
    
    // Interactive colors
    pub const BUTTON_DEFAULT: Color = Color::srgb(0.2, 0.2, 0.2);
    pub const BUTTON_HOVER: Color = Color::srgb(0.25, 0.25, 0.25);
    pub const BUTTON_SELECTED: Color = Color::srgb(0.3, 0.4, 0.5);
    pub const BUTTON_PRESSED: Color = Color::srgb(0.45, 0.45, 0.45);
    
    // Expansion button colors
    pub const EXPANSION_BUTTON_DEFAULT: Color = Color::srgb(0.3, 0.3, 0.3);
    pub const EXPANSION_BUTTON_HOVER: Color = Color::srgb(0.4, 0.4, 0.4);
    pub const EXPANSION_BUTTON_PRESSED: Color = Color::srgb(0.45, 0.45, 0.45);
}

/// Convenience constant for easy access
pub const DARK_THEME: DarkTheme = DarkTheme;

/// Standard font sizes used throughout the editor
pub struct FontSizes;

impl FontSizes {
    pub const HEADER: f32 = 15.0;
    pub const BODY: f32 = 13.0;
    pub const SMALL: f32 = 12.0;
    pub const BUTTON: f32 = 12.0;
}

/// Standard spacing values
pub struct Spacing;

impl Spacing {
    pub const TINY: f32 = 2.0;
    pub const SMALL: f32 = 4.0;
    pub const MEDIUM: f32 = 8.0;
    pub const LARGE: f32 = 12.0;
    pub const XLARGE: f32 = 16.0;
    pub const XXLARGE: f32 = 24.0;
}
