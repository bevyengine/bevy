use bevy_ecs::prelude::*;
use bevy_render::render_phase::RenderPhase;

use crate::prelude::CameraUi;

use super::TransparentUi;

/// Inserts the [`RenderPhase`] into the UI camera
pub fn extract_ui_camera_phases(mut commands: Commands, cameras_ui: Query<Entity, With<CameraUi>>) {
    for entity in cameras_ui.iter() {
        commands
            .get_or_spawn(entity)
            .insert(RenderPhase::<TransparentUi>::default());
    }
}
