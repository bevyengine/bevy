use super::{prepare::DlssRenderContext, Dlss, DlssFeature};
use bevy_camera::{Camera, MainPassResolutionOverride, Projection};
use bevy_ecs::{
    query::{Has, With},
    system::{Commands, Query, ResMut},
};
use bevy_render::{sync_world::RenderEntity, view::Hdr, MainWorld};

pub fn extract_dlss<F: DlssFeature>(
    mut commands: Commands,
    mut main_world: ResMut<MainWorld>,
    cleanup_query: Query<Has<Dlss<F>>>,
) {
    let mut cameras_3d = main_world
        .query_filtered::<(RenderEntity, &Camera, &Projection, Option<&mut Dlss<F>>), With<Hdr>>();

    for (entity, camera, camera_projection, mut dlss) in cameras_3d.iter_mut(&mut main_world) {
        let has_perspective_projection = matches!(camera_projection, Projection::Perspective(_));
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if dlss.is_some() && camera.is_active && has_perspective_projection {
            entity_commands.insert(dlss.as_deref().unwrap().clone());
            dlss.as_mut().unwrap().reset = false;
        } else if cleanup_query.get(entity) == Ok(true) {
            entity_commands.remove::<(Dlss<F>, DlssRenderContext<F>, MainPassResolutionOverride)>();
        }
    }
}
