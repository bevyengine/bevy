use bevy_ecs::prelude::*;
use bevy_render::{camera::ActiveCameras, render_phase::RenderPhase};

/// The name of the UI camera
pub const CAMERA_UI: &str = "camera_ui";

/// Inserts the [`RenderPhase`] into the UI camera
pub fn extract_ui_camera_phases(mut commands: Commands, active_cameras: Res<ActiveCameras>) {
    if let Some(camera_ui) = active_cameras.get(CAMERA_UI) {
        if let Some(entity) = camera_ui.entity {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<super::TransparentUi>::default());
        }
    }
}
