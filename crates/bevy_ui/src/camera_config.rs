//! Configuration for cameras related to UI.

use bevy_ecs::component::Component;
use bevy_ecs::prelude::With;
use bevy_render::camera::Camera;
use bevy_render::extract_component::ExtractComponent;

/// Configuration for cameras related to UI.
///
/// When a [`Camera`] doesn't have the [`UiCameraConfig`] component,
/// it will display the UI by default.
///
#[derive(Component, Clone, ExtractComponent)]
#[extract_component_filter(With<Camera>)]
pub struct UiCameraConfig {
    /// Whether to output UI to this camera view.
    ///
    /// When a `Camera` doesn't have the [`UiCameraConfig`] component,
    /// it will display the UI by default.
    pub show_ui: bool,
}

impl Default for UiCameraConfig {
    fn default() -> Self {
        Self { show_ui: true }
    }
}
