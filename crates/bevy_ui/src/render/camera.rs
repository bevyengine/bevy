use bevy_ecs::prelude::*;
use bevy_render::{camera::ActiveCamera, render_phase::RenderPhase};

use crate::prelude::CameraUi;

use super::TransparentUi;

/// Inserts the [`RenderPhase`] into the UI camera
pub fn extract_ui_camera_phases(
    mut commands: Commands,
    active_camera: Res<ActiveCamera<CameraUi>>,
) {
    if let Some(entity) = active_camera.get() {
        commands
            .get_or_spawn(entity)
            .insert(RenderPhase::<TransparentUi>::default());
    }
}
