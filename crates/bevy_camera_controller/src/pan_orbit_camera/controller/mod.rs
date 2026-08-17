//! Camera controller implementation.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

pub mod component;
pub mod inputs;
pub mod momentum;
pub mod motion;
pub mod projections;
pub mod smoothing;
pub mod transform_adapter;
pub mod zoom;

/// Adds [`bevy_editor_cam`](crate) functionality without an input plugin or any extensions. This
/// requires an input plugin to function! If you don't use the [`crate::input::DefaultInputPlugin`],
/// you will need to provide your own.
pub struct MinimalPanOrbitCameraPlugin;

impl Plugin for MinimalPanOrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<transform_adapter::TransformAdapter>()
            .add_systems(
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
