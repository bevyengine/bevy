//! Camera controller implementation.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

pub mod component;
pub mod inputs;
pub mod momentum;
pub mod motion;
pub mod projections;
pub mod smoothing;
pub mod transform_utils;
pub mod zoom;

/// Adds [`PanOrbitCamera`](crate::pan_orbit_camera::prelude::component::PanOrbitCamera) functionality without an input plugin or any extensions. This
/// requires an input plugin to function! You need to provide your own input plugin. You can see an example for how to connect this at `examples/camera/pan_orbit_camera_custom_input_plugin.rs`
pub struct MinimalPanOrbitCameraPlugin;

impl Plugin for MinimalPanOrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                component::PanOrbitCamera::update_camera_positions,
                projections::update_orthographic,
                // Technically `update_perspective` does not alter the camera
                // position, but the other two systems above do, so I'm putting
                // them all in the SyncCameraPosition group.
                projections::update_perspective,
            )
                .chain()
                .after(bevy_picking::PickingSystems::Last)
                .in_set(crate::pan_orbit_camera::SyncCameraPosition),
        );
    }
}
