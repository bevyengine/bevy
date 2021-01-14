use super::CameraProjection;
use bevy_app::prelude::EventReader;
use bevy_ecs::{Added, Component, Entity, Query, QuerySet, Res};
use bevy_math::Mat4;
use bevy_reflect::{Reflect, ReflectComponent};
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};

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

#[derive(Debug)]
pub enum DepthCalculation {
    Distance,
    ZDifference,
}

impl Default for DepthCalculation {
    fn default() -> Self {
        DepthCalculation::Distance
    }
}

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
    // handle resize events. latest events are handled first because we only want to resize each window once
    for event in window_resized_events.iter().rev() {
        if changed_window_ids.contains(&event.id) {
            continue;
        }

        changed_window_ids.push(event.id);
    }

    // handle resize events. latest events are handled first because we only want to resize each window once
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
            if changed_window_ids.contains(&window.id()) || added_cameras.contains(&entity) {
                camera_projection.update(window.width(), window.height());
                camera.projection_matrix = camera_projection.get_projection_matrix();
                camera.depth_calculation = camera_projection.depth_calculation();
            }
        }
    }
}
