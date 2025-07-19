//! # Editor UI Widgets
//!
//! This module provides a collection of reusable UI widgets specifically designed
//! for editor interfaces. The widgets integrate with Bevy's native UI system and
//! bevy_core_widgets for consistent behavior and performance.
//!
//! ## Available Widgets
//!
//! - **ScrollViewBuilder**: High-level scrollable container with built-in styling
//! - **CoreScrollArea**: Low-level scroll component for custom implementations  
//! - **ExpansionButton**: Collapsible content with expand/collapse functionality
//! - **BasicPanel**: Simple panel with header and content area
//! - **ListView**: Generic list widget with selection support
//!
//! ## Integration
//!
//! All widgets are designed to work seamlessly with:
//! - Bevy's native UI components (Node, Button, Text, etc.)
//! - bevy_core_widgets scrolling system
//! - Editor theme system for consistent styling
//! - Event-driven architecture using Bevy observers
//!
//! ## Usage
//!
//! Widgets can be used individually or through the main `WidgetsPlugin`:
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use bevy_editor::widgets::WidgetsPlugin;
//!
//! App::new()
//!     .add_plugins(WidgetsPlugin)
//!     .run();
//! ```

pub mod expansion_button;
pub mod simple_panel;
pub mod core_scroll_area;
pub mod scroll_view;

// Temporarily disabled complex widgets that need more work to compile with current Bevy
// pub mod scrollable_area;
// pub mod panel;
pub mod list_view;
// pub mod theme;

pub use expansion_button::*;
pub use simple_panel::{BasicPanel, spawn_basic_panel};
pub use core_scroll_area::*;
pub use scroll_view::*;
pub use list_view::*;

// Basic theme support
#[derive(Clone)]
pub struct EditorTheme {
    pub background_primary: bevy::prelude::Color,
    pub text_primary: bevy::prelude::Color,
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self {
            background_primary: bevy::prelude::Color::srgb(0.1, 0.1, 0.1),
            text_primary: bevy::prelude::Color::WHITE,
        }
    }
}

use bevy::prelude::*;

/// Plugin that provides reusable UI widgets
#[derive(Default)]
pub struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExpansionButtonPlugin,
            core_scroll_area::CoreScrollAreaPlugin,
            scroll_view::ScrollViewPlugin,
        ));
    }
}

/// Legacy plugin for backwards compatibility
#[derive(Default)]
pub struct ExpansionButtonPlugin;

impl Plugin for ExpansionButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, expansion_button::handle_expansion_clicks);
    }
}
