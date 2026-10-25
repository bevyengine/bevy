use super::{prepare::DlssRenderContext, Dlss, DlssFeature};
use crate::ray_reconstruction::RayReconstructionDenoiser;
use bevy_camera::{Camera, Hdr, MainPassResolutionOverride, Projection};
use bevy_ecs::{
    query::{Has, With},
    system::{Commands, Query, ResMut},
};
use bevy_render::{sync_world::RenderEntity, MainWorld};

pub fn extract_dlss<F: DlssFeature>(
    mut commands: Commands,
    mut main_world: ResMut<MainWorld>,
    cleanup_query: Query<Has<Dlss<F>>>,
) {
    let mut cameras_3d = main_world
        .query_filtered::<(RenderEntity, &Camera, &Projection, Option<&mut Dlss<F>>), With<Hdr>>();

    for (entity, camera, camera_projection, mut dlss) in cameras_3d.iter_mut(&mut main_world) {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if dlss.is_some() && camera.is_active && camera_projection.is_perspective() {
            entity_commands.insert(dlss.as_deref().unwrap().clone());
            // Inserted here, not in prepare: Solari allocates guide buffers in `PrepareResources`,
            // which runs before `PrepareViews`.
            if F::requests_ray_reconstruction_guides() {
                entity_commands.insert(RayReconstructionDenoiser);
            }
            dlss.as_mut().unwrap().reset = false;
        } else if cleanup_query.get(entity) == Ok(true) {
            entity_commands.remove::<(
                Dlss<F>,
                DlssRenderContext<F>,
                MainPassResolutionOverride,
                RayReconstructionDenoiser,
            )>();
        }
    }
}
