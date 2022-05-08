mod bundle;
#[allow(clippy::module_inception)]
mod camera;
mod projection;

pub use bundle::*;
pub use camera::*;
pub use projection::*;

use crate::{
    primitives::Aabb,
    view::{ComputedVisibility, Visibility, VisibleEntities},
};
use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::schedule::ParallelSystemDescriptorCoercion;
use bevy_window::ModifiesWindows;

#[derive(Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera>()
            .register_type::<Visibility>()
            .register_type::<ComputedVisibility>()
            .register_type::<OrthographicProjection>()
            .register_type::<PerspectiveProjection>()
            .register_type::<VisibleEntities>()
            .register_type::<WindowOrigin>()
            .register_type::<ScalingMode>()
            .register_type::<DepthCalculation>()
            .register_type::<Aabb>()
            .register_type::<Camera3d>()
            .register_type::<Camera2d>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::camera::camera_system::<OrthographicProjection>.after(ModifiesWindows),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::camera::camera_system::<PerspectiveProjection>.after(ModifiesWindows),
            )
            .add_plugin(CameraTypePlugin::<Camera3d>::default())
            .add_plugin(CameraTypePlugin::<Camera2d>::default());
    }
}
