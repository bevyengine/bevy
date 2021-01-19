use super::{Camera, DepthCalculation};
use bevy_ecs::Res;
use bevy_math::{Mat4, Vec2, Vec3};
use bevy_reflect::{Reflect, ReflectComponent, ReflectDeserialize};
use bevy_transform::components::GlobalTransform;
use bevy_window::Windows;
use serde::{Deserialize, Serialize};

pub trait CameraProjection {
    fn get_projection_matrix(&self) -> Mat4;
    fn update(&mut self, width: f32, height: f32);
    fn depth_calculation(&self) -> DepthCalculation;
}

#[derive(Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct PerspectiveProjection {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl CameraProjection for PerspectiveProjection {
    fn get_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }

    fn update(&mut self, width: f32, height: f32) {
        self.aspect_ratio = width / height;
    }

    fn depth_calculation(&self) -> DepthCalculation {
        DepthCalculation::Distance
    }
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        PerspectiveProjection {
            fov: std::f32::consts::PI / 4.0,
            near: 1.0,
            far: 1000.0,
            aspect_ratio: 1.0,
        }
    }
}

// TODO: make this a component instead of a property
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect_value(Serialize, Deserialize)]
pub enum WindowOrigin {
    Center,
    BottomLeft,
}

#[derive(Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct OrthographicProjection {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
    pub window_origin: WindowOrigin,
}

impl CameraProjection for OrthographicProjection {
    fn get_projection_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        )
    }

    fn update(&mut self, width: f32, height: f32) {
        match self.window_origin {
            WindowOrigin::Center => {
                let half_width = width / 2.0;
                let half_height = height / 2.0;
                self.left = -half_width;
                self.right = half_width;
                self.top = half_height;
                self.bottom = -half_height;
            }
            WindowOrigin::BottomLeft => {
                self.left = 0.0;
                self.right = width;
                self.top = height;
                self.bottom = 0.0;
            }
        }
    }

    fn depth_calculation(&self) -> DepthCalculation {
        DepthCalculation::ZDifference
    }
}

impl Default for OrthographicProjection {
    fn default() -> Self {
        OrthographicProjection {
            left: 0.0,
            right: 0.0,
            bottom: 0.0,
            top: 0.0,
            near: 0.0,
            far: 1000.0,
            window_origin: WindowOrigin::Center,
        }
    }
}

/// Given coordinates in world space, use the camera and window information to compute the
/// screen space coordinates.
pub fn world_to_screen(
    world_space_coords: &GlobalTransform,
    camera: (&Camera, &GlobalTransform),
    window_resource: &Res<Windows>,
) -> Option<Vec2> {
    let projection_matrix = camera.0.projection_matrix;
    let window = window_resource.get(camera.0.window)?;
    let window_size = Vec2::new(window.width(), window.height());
    // Build a transform to convert from world to NDC using camera data
    let world_to_ndc: Mat4 = projection_matrix * camera.1.compute_matrix().inverse();
    let ndc_space_coords: Vec3 = world_to_ndc.transform_point3(world_space_coords.translation);
    // NDC z-values outside of 0 < z < 1 are behind the camera and are thus not in screen space
    if ndc_space_coords.z < 0.0 || ndc_space_coords.z > 1.0 {
        return None;
    }
    // Once in NDC space, we can discard the z element and rescale x/y to fit the screen
    let screen_space_coords = (ndc_space_coords.truncate() + Vec2::one()) / 2.0 * window_size;
    Some(screen_space_coords)
}
