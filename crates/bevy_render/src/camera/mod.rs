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
    /// Name identifying the active 2D camera, used to determine the view(s) to render this frame.
    /// See [`ActiveCamera`] for details.
    ///
    /// [`ActiveCamera`]: crate::camera::ActiveCamera
    pub const CAMERA_2D: &'static str = "camera_2d";
    /// Name identifying the active 3D camera, used to determine the view(s) to render this frame.
    /// See [`ActiveCamera`] for details.
    ///
    /// [`ActiveCamera`]: crate::camera::ActiveCamera
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
        app.sub_app_mut(RenderApp)
            .init_resource::<ExtractedCameraNames>()
            .add_system_to_stage(RenderStage::Extract, extract_cameras);
    }
}

/// Resouce containing the collection of extracted camera names and their entity.
///
/// The resource is populated each frame, and contains a map of the camera name and entity
/// on which the source [`Camera`] is attached.
#[derive(Default)]
pub struct ExtractedCameraNames {
    /// Map of a [`Camera::name`] to the [`Entity`] on which the [`Camera`] component is.
    pub entities: HashMap<String, Entity>,
}

/// Camera data extracted for rendering.
///
/// This component is created automatically during the [`RenderStage::Extract`] stage
/// for each [`Camera`] component with an associated window, and for which a [`GlobalTransform`]
/// and [`VisibleEntities`] components are present on the same [`Entity`] as the [`Camera`].
/// The created component is attached to that same entity.
#[derive(Component, Debug)]
pub struct ExtractedCamera {
    /// The [`Camera::window`] of the source [`Camera`] this data was extracted from.
    pub window_id: WindowId,
    /// The [`Camera::name`] of the source [`Camera`] this data was extracted from.
    pub name: Option<String>,
}

/// System to extract cameras and views.
///
/// This system runs during the [`RenderStage::Extract`] stage to extract the cameras with
/// an associated window, and create an [`ExtractedCamera`] and [`ExtractedView`] for each
/// camera, to be consumed by later render stages. Only entities with a [`Camera`], a
/// [`GlobalTransform`], and a [`VisibleEntities`] are considered.
///
/// The system also populates the [`ExtractedCameraNames`] collection with the names of the
/// extracted cameras.
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
            if let Some(window) = windows.get(camera.window) {
                entities.insert(name.clone(), entity);
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
