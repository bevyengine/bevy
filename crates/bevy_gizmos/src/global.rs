use std::sync::Mutex;

use bevy_app::{App, Last, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;

use crate::{
    config::DefaultGizmoConfigGroup,
    gizmos::{GizmoBuffer, Gizmos},
    GizmoMeshSystems,
};

static GLOBAL_GIZMO: Mutex<GizmoBuffer<DefaultGizmoConfigGroup, ()>> =
    Mutex::new(GizmoBuffer::new());

/// Lets you use bevy gizmos outside of systems.
pub struct GlobalGizmosPlugin;

impl Plugin for GlobalGizmosPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, flush_global_gizmos.before(GizmoMeshSystems));
    }
}

fn flush_global_gizmos(mut gizmos: Gizmos) {
    let mut buffer = GizmoBuffer::new();
    {
        core::mem::swap(&mut buffer, &mut GLOBAL_GIZMO.lock().unwrap());
    }
    gizmos.strip_positions.extend(buffer.strip_positions);
    gizmos.strip_colors.extend(buffer.strip_colors);
    gizmos.list_positions.extend(buffer.list_positions);
    gizmos.list_colors.extend(buffer.list_colors);
}

/// A global gizmo context for use outside of bevy systems.
///
/// # Example
/// ```
/// # use bevy_gizmos::prelude::*;
/// # use bevy_math::prelude::*;
/// # use bevy_color::palettes::basic::WHITE;
/// fn draw() {
///     gizmo().sphere(Isometry3d::IDENTITY, 0.5, WHITE);
/// }
/// ```
pub fn gizmo() -> impl core::ops::DerefMut<Target = GizmoBuffer<DefaultGizmoConfigGroup, ()>> {
    GLOBAL_GIZMO.lock().unwrap()
}
