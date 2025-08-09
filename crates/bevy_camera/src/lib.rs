#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
mod camera;
mod clear_color;
mod components;
pub mod primitives;
mod projection;
pub mod visibility;

pub use camera::*;
pub use clear_color::*;
pub use components::*;
pub use projection::*;

use bevy_app::{App, Plugin};

#[derive(Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClearColor>().add_plugins((
            CameraProjectionPlugin,
            visibility::VisibilityPlugin,
            visibility::VisibilityRangePlugin,
        ));
    }
}

/// The camera prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        visibility::{InheritedVisibility, ViewVisibility, Visibility},
        Camera, Camera2d, Camera3d, ClearColor, ClearColorConfig, OrthographicProjection,
        PerspectiveProjection, Projection,
    };
}
