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
        app.register_type::<Camera>()
            .register_type::<ClearColor>()
            .register_type::<CameraMainTextureUsages>()
            .register_type::<Exposure>()
            .register_type::<MainPassResolutionOverride>()
            .register_type::<primitives::Aabb>()
            .register_type::<primitives::CascadesFrusta>()
            .register_type::<primitives::CubemapFrusta>()
            .register_type::<primitives::Frustum>()
            .init_resource::<ClearColor>()
            .add_plugins((
                CameraProjectionPlugin,
                visibility::VisibilityPlugin,
                visibility::VisibilityRangePlugin,
            ));
    }
}
