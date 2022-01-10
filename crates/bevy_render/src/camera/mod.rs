mod bundle;
#[allow(clippy::module_inception)]
mod camera;
mod projection;

pub use bundle::*;
pub use camera::*;
pub use projection::*;

use crate::{
    prelude::Image,
    primitives::Aabb,
    view::{ComputedVisibility, ExtractedView, Visibility, VisibleEntities},
    RenderApp, RenderStage,
};
use bevy_app::{App, CoreStage, Plugin};
use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_math::UVec2;
use bevy_transform::components::GlobalTransform;
use bevy_window::Windows;

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
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::camera::camera_system::<OrthographicProjection>,
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::camera::camera_system::<PerspectiveProjection>,
            );
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_system_to_stage(RenderStage::Extract, extract_cameras::<Camera3d>)
                .add_system_to_stage(RenderStage::Extract, extract_cameras::<Camera2d>);
        }
    }
}

#[derive(Component, Debug)]
pub struct ExtractedCamera {
    pub target: RenderTarget,
    pub physical_size: Option<UVec2>,
}

pub fn extract_cameras<M: Component + Default>(
    mut commands: Commands,
    windows: Res<Windows>,
    images: Res<Assets<Image>>,
    query: Query<(Entity, &Camera, &GlobalTransform, &VisibleEntities), With<M>>,
) {
    for (entity, camera, transform, visible_entities) in query.iter() {
        if let Some(size) = camera.target.get_physical_size(&windows, &images) {
            commands.get_or_spawn(entity).insert_bundle((
                ExtractedCamera {
                    target: camera.target.clone(),
                    physical_size: camera.target.get_physical_size(&windows, &images),
                },
                ExtractedView {
                    projection: camera.projection_matrix,
                    transform: *transform,
                    width: size.x.max(1),
                    height: size.y.max(1),
                    near: camera.near,
                    far: camera.far,
                },
                visible_entities.clone(),
                M::default(),
            ));
        }
    }
}
