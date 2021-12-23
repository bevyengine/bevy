mod active_cameras;
mod bundle;
#[allow(clippy::module_inception)]
mod camera;
mod projection;

pub use active_cameras::*;
use bevy_asset::Assets;
use bevy_math::UVec2;
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use bevy_window::Windows;
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
use bevy_ecs::prelude::*;

#[derive(Default)]
pub struct CameraPlugin;

impl CameraPlugin {
    pub const CAMERA_2D: &'static str = "camera_2d";
    pub const CAMERA_3D: &'static str = "camera_3d";
}

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        let mut active_cameras = ActiveCameras::default();
        active_cameras.add(Self::CAMERA_2D);
        active_cameras.add(Self::CAMERA_3D);
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
            .insert_resource(active_cameras)
            .add_system_to_stage(CoreStage::PostUpdate, crate::camera::active_cameras_system)
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
                .init_resource::<ExtractedCameraNames>()
                .add_system_to_stage(RenderStage::Extract, extract_cameras);
        }
    }
}

#[derive(Default)]
pub struct ExtractedCameraNames {
    pub entities: HashMap<String, Entity>,
}

#[derive(Component, Debug)]
pub struct ExtractedCamera {
    pub target: RenderTarget,
    pub name: Option<String>,
    pub physical_size: Option<UVec2>,
}

fn extract_cameras(
    mut commands: Commands,
    active_cameras: Res<ActiveCameras>,
    windows: Res<Windows>,
    images: Res<Assets<Image>>,
    query: Query<(Entity, &Camera, &GlobalTransform, &VisibleEntities)>,
) {
    let mut entities = HashMap::default();
    for camera in active_cameras.iter() {
        let name = &camera.name;
        if let Some((entity, camera, transform, visible_entities)) =
            camera.entity.and_then(|e| query.get(e).ok())
        {
            if let Some(size) = camera.target.get_physical_size(&windows, &images) {
                entities.insert(name.clone(), entity);
                commands.get_or_spawn(entity).insert_bundle((
                    ExtractedCamera {
                        target: camera.target.clone(),
                        name: camera.name.clone(),
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
                ));
            }
        }
    }

    commands.insert_resource(ExtractedCameraNames { entities });
}
