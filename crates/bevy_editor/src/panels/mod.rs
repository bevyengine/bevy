//! # Editor Panels
//!
//! This module provides the main UI panels that make up the editor interface.
//! Each panel is implemented as a separate plugin for modular usage and can
//! be combined to create custom editor layouts.
//!
//! ## Available Panels
//!
//! - **EntityListPlugin**: Entity browser with selection and filtering
//! - **ComponentInspectorPlugin**: Detailed component viewer with expansion
//!
//! ## Panel Architecture
//!
//! Each panel follows a consistent pattern:
//! - Self-contained plugin with its own systems and resources
//! - Event-driven communication between panels
//! - Responsive UI that adapts to different screen sizes
//! - Integration with the editor's theme system
//!
//! ## Usage
//!
//! Panels can be used individually for custom editor layouts:
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use bevy_editor::panels::{EntityListPlugin, ComponentInspectorPlugin};
//!
//! App::new()
//!     .add_plugins((EntityListPlugin, ComponentInspectorPlugin))
//!     .run();
//! ```

pub mod entity_list;
pub mod component_inspector;

pub use entity_list::*;
pub use component_inspector::{
    ComponentInspector, ComponentInspectorContent, ComponentInspectorText, ComponentInspectorScrollArea,
    parse_component_fields, handle_component_data_fetched
};

// Re-export commonly used types
pub use crate::remote::types::{EditorState, ComponentDisplayState, ComponentField};

use bevy::prelude::*;

/// Plugin for component inspector panel  
#[derive(Default)]
pub struct ComponentInspectorPlugin;

impl Plugin for ComponentInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(component_inspector::handle_component_data_fetched)
           .init_resource::<ComponentDisplayState>();
    }
}
