//! Flying camera plugin for the game engine Bevy

mod camera;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::CameraController;
}

pub use camera::*;

use bevy_app::prelude::*;

/// Simple flying camera plugin.
/// In order to function, the [`CameraController`] component should be attached to the camera entity.
#[derive(Default)]
pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(camera_controller).add_system(print_controls);
    }
}
