use crate::camera::CameraProjection;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    prelude::{Changed, DetectChanges, QueryState},
    reflect::ReflectComponent,
    system::{QuerySet, Res},
};
use bevy_math::{Mat4, Vec2, Vec3};
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_transform::components::GlobalTransform;
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use serde::{Deserialize, Serialize};

#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
pub struct Camera {
    pub projection_matrix: Mat4,
    pub name: Option<String>,
    #[reflect(ignore)]
    pub window: WindowId,
    #[reflect(ignore)]
    pub depth_calculation: DepthCalculation,
    pub near: f32,
    pub far: f32,
    pub viewport: Option<Viewport>,
}

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// In range `0..=1`
    pub min_depth: f32,
    /// In range `0..=1`
    pub max_depth: f32,

    /// Whether `x`, `y`, `w` and `h` should be in range `0..=1` or in pixels
    pub scaling_mode: ViewportScalingMode,
}
impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
            min_depth: 0.0,
            max_depth: 1.0,
            scaling_mode: ViewportScalingMode::Normalized,
        }
    }
}

impl Viewport {
    pub fn scaled_pos(&self, target_size: Vec2) -> Vec2 {
        let pos = Vec2::new(self.x, self.y);
        match self.scaling_mode {
            ViewportScalingMode::Normalized => pos * target_size,
            ViewportScalingMode::Pixels => pos,
        }
    }
    pub fn scaled_size(&self, target_size: Vec2) -> Vec2 {
        let size = Vec2::new(self.w, self.h);
        match self.scaling_mode {
            ViewportScalingMode::Normalized => size * target_size,
            ViewportScalingMode::Pixels => size,
        }
    }
}

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
pub enum ViewportScalingMode {
    /// `x`, `y`, `w` and `h` are in `0..=1`
    Normalized,
    /// Pixel units
    Pixels,
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

#[allow(clippy::type_complexity)]
pub fn camera_system<T: CameraProjection + Component>(
    mut window_resized_events: EventReader<WindowResized>,
    mut window_created_events: EventReader<WindowCreated>,
    windows: Res<Windows>,
    mut queries: QuerySet<(
        QueryState<(Entity, &mut Camera, &mut T)>,
        QueryState<Entity, Changed<Camera>>,
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
    for (entity, mut camera, mut camera_projection) in queries.q0().iter_mut() {
        if let Some(window) = windows.get(camera.window) {
            if changed_window_ids.contains(&window.id())
                || added_cameras.contains(&entity)
                || camera_projection.is_changed()
            {
                let target_size = Vec2::new(window.width(), window.height());
                let size = camera
                    .viewport
                    .as_ref()
                    .map(|viewport| viewport.scaled_size(target_size))
                    .unwrap_or(target_size);
                camera_projection.update(size.x, size.y);
                camera.projection_matrix = camera_projection.get_projection_matrix();
                camera.depth_calculation = camera_projection.depth_calculation();
            }
        }
    }
}
