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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default)]
pub enum InspectorState {
    #[default]
    Inactive,
    Active,
}

#[derive(Resource, Debug)]
pub struct InspectorData {
    pub selected_entity: Option<Entity>,
    pub inspector_window: Option<Entity>,
    pub inspector_camera: Option<Entity>,
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

#[derive(Resource, Debug)]
pub struct InspectorConfig {
    pub toggle_key: KeyCode,
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