use super::Camera;
use bevy_ecs::{
    entity::Entity,
    system::{Query, ResMut},
};
use bevy_utils::HashMap;

/// A reference to the entity holding the camera to use for a given content type.
///
/// This acts as a link between:
/// - an [`ExtractedCamera`], that is, a camera found to be valid for the current frame as
///   analyzed during the [`RenderStage::Extract`] stage, and indirectly the underlying
///   [`Camera`] it was extracted from;
/// - a set of render phases used to render a specific kind of content (2D or 3D).
///
/// A set of [`ActiveCamera`] for each supported kind of content is pre-built at startup and
/// added to the [`ActiveCameras`] resource. Their `entity` field is then updated each frame
/// to point to the entity holding the [`Camera`] to use for that content. See [`ActiveCameras`]
/// for details.
///
/// Note that the name "active camera" is a misnomer, and does not mean "a Camera which is active",
/// as multiple [`Camera`] components can be added to the same world, and the [`Camera`] component
/// itself has no concept of being "active" or not. Instead, it refers to the camera being the one
/// selected for rendering the content type defined by `name`.
///
/// [`ExtractedCamera`]: super::ExtractedCamera
/// [`RenderStage::Extract`]: crate::RenderStage::Extract
#[derive(Debug, Default)]
pub struct ActiveCamera {
    /// Name of the active camera, which reference the content type to render. Valid names are
    /// [`CameraPlugin::CAMERA_2D`] and [`CameraPlugin::CAMERA_3D`] only.
    ///
    /// [`CameraPlugin::CAMERA_2D`]: super::CameraPlugin::CAMERA_2D
    /// [`CameraPlugin::CAMERA_3D`]: super::CameraPlugin::CAMERA_3D
    pub name: String,
    /// Entity holding the [`Camera`] to use to render the content type. This is updated each
    /// frame automatically during the [`CoreStage::PostUpdate`] stage by the [`active_cameras_system`]
    /// system.
    ///
    /// [`CoreStage::PostUpdate`]: bevy_app::CoreStage::PostUpdate
    pub entity: Option<Entity>,
}

/// Resource holding all active cameras.
///
/// The collection only contains 2 predefined items added at startup:
/// - The active 2D camera, with name [`CameraPlugin::CAMERA_2D`], used to render 2D content.
/// - The active 3D camera, with name [`CameraPlugin::CAMERA_3D`], used to render 3D content.
///
/// The resource is updated by the internal core renderer during the [`CoreStage::PostUpdate`]
/// stage, and further consumed during the [`RenderStage::Extract`] stage to insert on the
/// `entity` the various [`RenderPhase`] associated with it:
///
/// - For the active 2D camera, the `Transparent2d` phase.
/// - For the active 3D camera, the `Opaque3d`, `AlphaMask3d`, and `Transparent3d` phases.
///
/// [`CameraPlugin::CAMERA_2D`]: super::CameraPlugin::CAMERA_2D
/// [`CameraPlugin::CAMERA_3D`]: super::CameraPlugin::CAMERA_3D
/// [`CoreStage::PostUpdate`]: bevy_app::CoreStage::PostUpdate
/// [`RenderStage::Extract`]: crate::RenderStage::Extract
/// [`RenderPhase`]: crate::render_phase::RenderPhase
#[derive(Debug, Default)]
pub struct ActiveCameras {
    cameras: HashMap<String, ActiveCamera>,
}

impl ActiveCameras {
    pub fn add(&mut self, name: &str) {
        self.cameras.insert(
            name.to_string(),
            ActiveCamera {
                name: name.to_string(),
                ..Default::default()
            },
        );
    }

    pub fn get(&self, name: &str) -> Option<&ActiveCamera> {
        self.cameras.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ActiveCamera> {
        self.cameras.get_mut(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<ActiveCamera> {
        self.cameras.remove(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ActiveCamera> {
        self.cameras.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ActiveCamera> {
        self.cameras.values_mut()
    }
}

/// System to update [`ActiveCameras`] each frame.
///
/// System running during the [`CoreStage::PostUpdate`] stage to determine which
/// of all the existing [`Camera`] components added to the world need to be used
/// to render the various types of content. The system updates the [`ActiveCameras`]
/// resource, setting the [`ActiveCamera::entity`] field of each entry to the
/// entity holding the selected [`Camera`] component.
///
/// [`CoreStage::PostUpdate`]: bevy_app::CoreStage::PostUpdate
pub fn active_cameras_system(
    mut active_cameras: ResMut<ActiveCameras>,
    query: Query<(Entity, &Camera)>,
) {
    for (name, active_camera) in active_cameras.cameras.iter_mut() {
        if active_camera
            .entity
            .map_or(false, |entity| query.get(entity).is_err())
        {
            active_camera.entity = None;
        }

        if active_camera.entity.is_none() {
            for (camera_entity, camera) in query.iter() {
                if let Some(ref current_name) = camera.name {
                    if current_name == name {
                        active_camera.entity = Some(camera_entity);
                    }
                }
            }
        }
    }
}
