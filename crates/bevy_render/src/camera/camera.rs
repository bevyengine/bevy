use super::CameraProjection;
use crate::surface::Viewport;
use bevy_ecs::{Changed, Component, Query};
use bevy_math::{Mat4, Vec2, Vec3};
use bevy_reflect::{Reflect, ReflectComponent, ReflectDeserialize};
use bevy_transform::components::GlobalTransform;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Reflect)]
#[reflect(Component)]
pub struct Camera {
    pub projection_matrix: Mat4,
    pub name: Option<String>,
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
        viewport: &Viewport,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Option<Vec2> {
        // Build a transform to convert from world to NDC using camera data
        let world_to_ndc: Mat4 =
            self.projection_matrix * camera_transform.compute_matrix().inverse();
        let ndc_space_coords: Vec3 = world_to_ndc.transform_point3(world_position);
        // NDC z-values outside of 0 < z < 1 are behind the camera and are thus not in screen space
        if ndc_space_coords.z < 0.0 || ndc_space_coords.z > 1.0 {
            return None;
        }
        // Once in NDC space, we can discard the z element and rescale x/y to fit the screen
        let screen_space_coords =
            viewport.origin() + (ndc_space_coords.truncate() + Vec2::one()) / 2.0 * viewport.size();
        Some(screen_space_coords)
    }
}

pub fn camera_system<T: CameraProjection + Component>(
    mut query: Query<(&mut Camera, &mut T, &Viewport), Changed<Viewport>>,
) {
    for (mut camera, mut camera_projection, viewport) in query.iter_mut() {
        let size = viewport.size();
        camera_projection.update(size.x, size.y);
        camera.projection_matrix = camera_projection.get_projection_matrix();
        camera.depth_calculation = camera_projection.depth_calculation();
    }
}
