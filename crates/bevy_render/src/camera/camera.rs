use super::CameraProjection;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    query::Added,
    reflect::ReflectComponent,
    system::{Query, QuerySet, Res},
};
use bevy_geometry::{Line, Plane};
use bevy_math::{Mat4, Vec2, Vec3};
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_transform::components::GlobalTransform;
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Reflect)]
#[reflect(Component)]
pub struct Camera {
    pub projection_matrix: Mat4,
    pub name: Option<String>,
    #[reflect(ignore)]
    pub window: WindowId,
    #[reflect(ignore)]
    pub depth_calculation: DepthCalculation,
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
        // NDC z-values outside of 0 < z < 1 are behind the camera and are thus not in screen space
        if ndc_space_coords.z < 0.0 || ndc_space_coords.z > 1.0 {
            return None;
        }
        // Once in NDC space, we can discard the z element and rescale x/y to fit the screen
        let screen_space_coords = (ndc_space_coords.truncate() + Vec2::ONE) / 2.0 * window_size;
        Some(screen_space_coords)
    }

    /// Given a position in screen space, compute the world-space line that corresponds to it.
    pub fn screen_to_world_line<W: AsRef<Windows>>(
        pos_screen: Vec2,
        windows: W,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Line {
        let camera_position = camera_transform.compute_matrix();
        let window = windows
            .as_ref()
            .get(camera.window)
            .unwrap_or_else(|| panic!("WindowId {} does not exist", camera.window));
        let screen_size = Vec2::from([window.width() as f32, window.height() as f32]);
        let projection_matrix = camera.projection_matrix;

        // Normalized device coordinate cursor position from (-1, -1, -1) to (1, 1, 1)
        let cursor_ndc = (pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0]);
        let cursor_pos_ndc_near: Vec3 = cursor_ndc.extend(-1.0);
        let cursor_pos_ndc_far: Vec3 = cursor_ndc.extend(1.0);

        // Use near and far ndc points to generate a ray in world space
        // This method is more robust than using the location of the camera as the start of
        // the ray, because ortho cameras have a focal point at infinity!
        let ndc_to_world: Mat4 = camera_position * projection_matrix.inverse();
        let cursor_pos_near: Vec3 = ndc_to_world.project_point3(cursor_pos_ndc_near);
        let cursor_pos_far: Vec3 = ndc_to_world.project_point3(cursor_pos_ndc_far);
        let ray_direction = cursor_pos_far - cursor_pos_near;
        Line::from_point_direction(cursor_pos_near, ray_direction)
        //Ray3d::new(cursor_pos_near, ray_direction)
    }

    /// Given a position in screen space and a plane in world space, compute what point on the plane the point in screen space corresponds to.
    /// In 2D, use `screen_to_point_2d`.
    pub fn screen_to_point_on_plane<W: AsRef<Windows>>(
        pos_screen: Vec2,
        plane: Plane,
        windows: W,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Option<Vec3> {
        let world_line = Self::screen_to_world_line(pos_screen, windows, camera, camera_transform);
        plane.intersection_line(&world_line)
    }

    /// Computes the world position for a given screen position.
    /// The output will always be on the XY plane with Z at zero. It is designed for 2D, but also works with a 3D camera.
    /// For more flexibility in 3D, consider `screen_to_point_on_plane`.
    pub fn screen_to_point_2d<W: AsRef<Windows>>(
        pos_screen: Vec2,
        windows: W,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Option<Vec3> {
        Self::screen_to_point_on_plane(
            pos_screen,
            Plane::from_point_normal(Vec3::new(0., 0., 0.), Vec3::new(0., 0., 1.)),
            windows,
            camera,
            camera_transform,
        )
    }
}

#[allow(clippy::type_complexity)]
pub fn camera_system<T: CameraProjection + Component>(
    mut window_resized_events: EventReader<WindowResized>,
    mut window_created_events: EventReader<WindowCreated>,
    windows: Res<Windows>,
    mut queries: QuerySet<(
        Query<(Entity, &mut Camera, &mut T)>,
        Query<Entity, Added<Camera>>,
    )>,
) {
    let mut changed_window_ids = Vec::new();
    // handle resize events. latest events are handled first because we only want to resize each
    // window once
    for event in window_resized_events.iter().rev() {
        if changed_window_ids.contains(&event.id) {
            continue;
        }

        changed_window_ids.push(event.id);
    }

    // handle resize events. latest events are handled first because we only want to resize each
    // window once
    for event in window_created_events.iter().rev() {
        if changed_window_ids.contains(&event.id) {
            continue;
        }

        changed_window_ids.push(event.id);
    }

    let mut added_cameras = vec![];
    for entity in &mut queries.q1().iter() {
        added_cameras.push(entity);
    }
    for (entity, mut camera, mut camera_projection) in queries.q0_mut().iter_mut() {
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
