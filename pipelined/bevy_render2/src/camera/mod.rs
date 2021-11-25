mod active_cameras;
mod bundle;
#[allow(clippy::module_inception)]
mod camera;
mod projection;

pub use active_cameras::*;
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use bevy_window::{WindowId, Windows};
pub use bundle::*;
pub use camera::*;
pub use projection::*;

use crate::{
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
        app.sub_app(RenderApp)
            .init_resource::<ExtractedCameraNames>()
            .add_system_to_stage(RenderStage::Extract, extract_cameras);
    }
}

#[derive(Default)]
pub struct ExtractedCameraNames {
    pub entities: HashMap<String, Entity>,
}

#[derive(Component, Debug)]
pub struct ExtractedCamera {
    pub window_id: WindowId,
    pub name: Option<String>,
}

fn extract_cameras(
    mut commands: Commands,
    active_cameras: Res<ActiveCameras>,
    windows: Res<Windows>,
    query: Query<(Entity, &Camera, &GlobalTransform, &VisibleEntities)>,
) {
    let mut entities = HashMap::default();
    for camera in active_cameras.iter() {
        let name = &camera.name;
        if let Some((entity, camera, transform, visible_entities)) =
            camera.entity.and_then(|e| query.get(e).ok())
        {
            entities.insert(name.clone(), entity);
            if let Some(window) = windows.get(camera.window) {
                commands.get_or_spawn(entity).insert_bundle((
                    ExtractedCamera {
                        window_id: camera.window,
                        name: camera.name.clone(),
                    },
                    ExtractedView {
                        projection: camera.projection_matrix,
                        transform: *transform,
                        width: window.physical_width().max(1),
                        height: window.physical_height().max(1),
                        near: camera.near,
                        far: camera.far,
                    },
                    visible_entities.clone(),
                ));
            }
        }
    }

    commands.insert_resource(ExtractedCameraNames { entities })
}
