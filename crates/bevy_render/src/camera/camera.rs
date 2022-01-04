use crate::camera::CameraProjection;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    prelude::{DetectChanges, QueryState},
    query::Added,
    reflect::ReflectComponent,
    system::{QuerySet, Res},
};
use bevy_math::{Mat4, Vec2, Vec3};
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_transform::components::GlobalTransform;
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use serde::{Deserialize, Serialize};

/// The defining component for camera entities, storing information about how and what to render
/// through this camera.
///
/// The [`Camera`] component is added to an entity to define the properties of the viewpoint from
/// which rendering occurs. It defines the position of the view to render, the projection method
/// to transform the 3D objects into a 2D image, as well as the window into which that image
/// is drawn.
///
/// Adding a camera is typically done by adding a bundle, either the [`OrthographicCameraBundle`]
/// or the [`PerspectiveCameraBundle`].
///
/// The internal Bevy renderer makes a distinction between 2D and 3D content, which are rendered
/// separately. To that end, at most two cameras should be added to the world, one per content.
/// The content type is determined by the camera name, which is automatically set as appropriate
/// by the various bundles.
///
/// [`OrthographicCameraBundle`]: crate::camera::OrthographicCameraBundle
/// [`PerspectiveCameraBundle`]: crate::camera::PerspectiveCameraBundle
#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
pub struct Camera {
    /// The projection matrix of the camera, transforming objects from the 3D camera space to the
    /// 2D screen space.
    ///
    /// If a [`CameraProjection`] component is present on the same entity as this camera, this
    /// projection matrix is automatically updated each frame based on that [`CameraProjection`]
    /// component.
    pub projection_matrix: Mat4,
    /// The camera name. This is used to identify the active 2D and 3D cameras, by assigning
    /// the [`CameraPlugin::CAMERA_2D`] and [`CameraPlugin::CAMERA_3D`] predefined names,
    /// respectively. See [`ActiveCameras`] for details.
    ///
    /// [`CameraPlugin::CAMERA_2D`]: super::CameraPlugin::CAMERA_2D
    /// [`CameraPlugin::CAMERA_3D`]: super::CameraPlugin::CAMERA_3D
    /// [`ActiveCameras`]: crate::camera::ActiveCameras
    pub name: Option<String>,
    /// The window into which this camera rendering is drawn.
    #[reflect(ignore)]
    pub window: WindowId,
    /// The depth calculation method used when rendering with this camera.
    #[reflect(ignore)]
    pub depth_calculation: DepthCalculation,
    /// Distance of the near clipping plane from the camera position.
    ///
    /// All fragments, produced by rendering the 3D objects, which are closer than this distance
    /// are clipped (hidden).
    ///
    /// This value is typically set to the furthest acceptable distance which still allows rendering
    /// a world without artifacts. For perspective projection, this value must be strictly positive.
    /// Keeping the `near`-to-`far` planes distance as short as possible prevents precision loss in the
    /// depth buffer, which can otherwise cause rendering artifacts.
    pub near: f32,
    /// Distance of the far clipping plane from the camera position.
    ///
    /// All fragments, produced by rendering the 3D objects, which are further away than this
    /// distance are clipped (hidden).
    ///
    /// This value is typically set to the closest acceptable distance which still allows objects being
    /// visible as far as intended. Keeping the `near`-to-`far` planes distance as short as possible
    /// prevents precision loss in the depth buffer, which can otherwise cause rendering artifacts.
    pub far: f32,
}

#[derive(Debug, Clone, Copy, Reflect, Serialize, Deserialize)]
#[reflect_value(Serialize, Deserialize)]
pub enum DepthCalculation {
    /// Pythagorean distance; works everywhere, more expensive to compute.
    Distance,
    /// Optimization for 2D; assuming the camera points towards -Z.
    ZDifference,
}

impl Default for DepthCalculation {
    fn default() -> Self {
        DepthCalculation::Distance
    }
}

impl Camera {
    /// Given a position in world space, use the camera to compute the screen space coordinates.
    pub fn world_to_screen(
        &self,
        windows: &Windows,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Option<Vec2> {
        let window = windows.get(self.window)?;
        let window_size = Vec2::new(window.width(), window.height());
        // Build a transform to convert from world to NDC using camera data
        let world_to_ndc: Mat4 =
            self.projection_matrix * camera_transform.compute_matrix().inverse();
        let ndc_space_coords: Vec3 = world_to_ndc.project_point3(world_position);
        // NDC z-values outside of 0 < z < 1 are outside the camera frustum and are thus not in screen space
        if ndc_space_coords.z < 0.0 || ndc_space_coords.z > 1.0 {
            return None;
        }
        // Once in NDC space, we can discard the z element and rescale x/y to fit the screen
        let screen_space_coords = (ndc_space_coords.truncate() + Vec2::ONE) / 2.0 * window_size;
        if !screen_space_coords.is_nan() {
            Some(screen_space_coords)
        } else {
            None
        }
    }
}

/// System in charge of updating a [`Camera`] when its window or projection change.
///
/// The system detects window creation and resize events to update the camera projection if
/// needed. It also queries any [`CameraProjection`] component associated with the same entity
/// as the [`Camera`] one, to automatically update the camera projection matrix.
///
/// The system function is generic over the camera projection type, and only instances of
/// [`OrthographicProjection`] and [`PerspectiveProjection`] are automatically added to
/// the app, running during the [`CoreStage::PostUpdate`] stage.
///
/// [`OrthographicProjection`]: crate::camera::OrthographicProjection
/// [`PerspectiveProjection`]: crate::camera::PerspectiveProjection
/// [`CoreStage::PostUpdate`]: bevy_app::CoreStage::PostUpdate
#[allow(clippy::type_complexity)]
pub fn camera_system<T: CameraProjection + Component>(
    mut window_resized_events: EventReader<WindowResized>,
    mut window_created_events: EventReader<WindowCreated>,
    windows: Res<Windows>,
    mut queries: QuerySet<(
        QueryState<(Entity, &mut Camera, &mut T)>,
        QueryState<Entity, Added<Camera>>,
    )>,
) {
    let mut changed_window_ids = Vec::new();

    // Collect all unique window IDs of changed windows by inspecting resized windows
    for event in window_resized_events.iter() {
        if changed_window_ids.contains(&event.id) {
            continue;
        }
        changed_window_ids.push(event.id);
    }

    // Collect all unique window IDs of changed windows by inspecting created windows
    for event in window_created_events.iter() {
        if changed_window_ids.contains(&event.id) {
            continue;
        }
        changed_window_ids.push(event.id);
    }

    // Collect all entities which have a new Camera component
    let mut added_cameras = vec![];
    for entity in &mut queries.q1().iter() {
        added_cameras.push(entity);
    }

    for (entity, mut camera, mut camera_projection) in queries.q0().iter_mut() {
        if let Some(window) = windows.get(camera.window) {
            if changed_window_ids.contains(&window.id())
                || added_cameras.contains(&entity)
                || camera_projection.is_changed()
            {
                camera_projection.update(window.width(), window.height());
                camera.projection_matrix = camera_projection.get_projection_matrix();
                camera.depth_calculation = camera_projection.depth_calculation();
            }
        }
    }
}
