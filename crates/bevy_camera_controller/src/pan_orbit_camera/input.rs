//! Provides a default input plugin for the camera. See [`DefaultInputPlugin`].

use bevy_app::prelude::*;
/// Default implementation of input plugin for [`PanOrbitCamera`](crate::pan_orbit_camera::controller::component::PanOrbitCamera)
pub struct DefaultInputPlugin;
impl Plugin for DefaultInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, || {
            bevy_log::error!("DefaultInputPlugin is not implemented! Use examples/camera/pan_orbit_camera_custom_input_plugin.rs for now.");
        });
    }
}
