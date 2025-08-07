//! Reusable UI widgets for Bevy
//!
//! This module contains high-quality, reusable UI widgets that are suitable for 
//! upstreaming into bevy_ui. Each widget is designed to be:
//!
//! - **Modular**: Self-contained with minimal dependencies
//! - **Performant**: Optimized for real-world usage
//! - **Extensible**: Easy to customize and extend
//! - **Well-documented**: Clear API and usage examples
//!
//! ## Available Widgets
//!
//! - [`SelectableText`]: Text selection and clipboard copy functionality
//! - [`VirtualScrolling`]: High-performance scrolling for large lists
//! - [`CollapsibleSection`]: Expandable/collapsible content sections
//!
//! ## Usage
//!
//! Each widget can be used independently by adding the appropriate systems
//! and components to your Bevy app:
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use bevy_dev_tools::widgets::{SelectableTextPlugin, VirtualScrollPlugin, CollapsibleSectionPlugin};
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins((
//!             SelectableTextPlugin,
//!             VirtualScrollPlugin,
//!             CollapsibleSectionPlugin,
//!         ))
//!         .run();
//! }
//! ```

pub mod selectable_text;
pub mod virtual_scrolling;
pub mod collapsible_section;

pub use selectable_text::*;
pub use virtual_scrolling::*;
pub use collapsible_section::*;

use bevy_app::{App, Plugin, Update};

/// Plugin that adds all reusable UI widgets
pub struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SelectableTextPlugin,
            VirtualScrollPlugin,
            CollapsibleSectionPlugin,
        ));
    }
}

/// Plugin for selectable text functionality
pub struct SelectableTextPlugin;

impl Plugin for SelectableTextPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<TextSelectionState>()
            .add_systems(Update, (
                handle_text_selection,
                sync_selectable_text_with_text,
            ));
    }
}

/// Plugin for virtual scrolling functionality
pub struct VirtualScrollPlugin;

impl Plugin for VirtualScrollPlugin {
    fn build(&self, _app: &mut App) {
        // Note: This is a generic plugin. Specific types need to be registered
        // when using virtual scrolling with concrete types.
        // Systems are added when specific virtual scroll instances are created.
        println!("VirtualScrollPlugin initialized - register specific types as needed");
    }
}