//! Entity/Component Inspector for Bevy applications.
//!
//! This module provides a comprehensive entity inspector that allows developers to inspect
//! the state of their application's World at runtime in a dedicated window.

use bevy_ecs::{entity::Entity, prelude::Resource};
use bevy_input::keyboard::KeyCode;
use bevy_state::prelude::*;

mod plugin;
mod systems;
mod ui;

pub use plugin::EntityInspectorPlugin;

/// The current state of the Entity Inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default)]
pub enum InspectorState {
    /// The inspector is not currently active/visible.
    #[default]
    Inactive,
    /// The inspector is currently active/visible.
    Active,
}

/// Internal resource that tracks the inspector's UI state and selected entity.
#[derive(Resource, Debug)]
pub struct InspectorData {
    /// The currently selected entity for component inspection.
    pub selected_entity: Option<Entity>,
    /// The inspector window entity (only used in separate window mode).
    pub inspector_window: Option<Entity>,
    /// The camera entity for the inspector window (only used in separate window mode).
    pub inspector_camera: Option<Entity>,
    /// The root UI entity for the inspector interface.
    pub ui_root: Option<Entity>,
}

impl Default for InspectorData {
    fn default() -> Self {
        Self {
            selected_entity: None,
            inspector_window: None,
            inspector_camera: None,
            ui_root: None,
        }
    }
}

/// Configuration for the Entity Inspector.
#[derive(Resource, Debug)]
pub struct InspectorConfig {
    /// The key used to toggle the inspector on/off.
    pub toggle_key: KeyCode,
    /// Whether to use overlay mode (true) or separate window mode (false).
    /// Overlay mode renders the inspector as an overlay on the main game window.
    /// Separate window mode opens a dedicated inspector window.
    pub use_overlay_mode: bool,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            toggle_key: KeyCode::F12,
            use_overlay_mode: true, // Default to overlay mode for better performance
        }
    }
}