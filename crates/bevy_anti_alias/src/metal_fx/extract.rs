use super::{prepare::MetalFxRenderContext, MetalFx, MetalFxFeature};
use crate::ray_reconstruction::RayReconstructionDenoiser;
use bevy_camera::{Camera, Hdr, MainPassResolutionOverride, Projection};
use bevy_ecs::{
    query::{Has, With},
    system::{Commands, Query, ResMut},
};
use bevy_render::{sync_world::RenderEntity, MainWorld};

pub fn extract_metal_fx<F: MetalFxFeature>(
    mut commands: Commands,
    mut main_world: ResMut<MainWorld>,
    cleanup_query: Query<Has<MetalFx<F>>>,
) {
    let mut cameras_3d = main_world
        .query_filtered::<(RenderEntity, &Camera, &Projection, Option<&mut MetalFx<F>>), With<Hdr>>(
        );

    for (entity, camera, camera_projection, mut metal_fx) in cameras_3d.iter_mut(&mut main_world) {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if metal_fx.is_some() && camera.is_active && camera_projection.is_perspective() {
            entity_commands.insert((
                metal_fx.as_deref().unwrap().clone(),
                RayReconstructionDenoiser,
            ));
            // Inserted here, not in prepare: Solari allocates guide buffers in `PrepareResources`,
            // which runs before `PrepareViews`.
            metal_fx.as_mut().unwrap().reset = false;
        } else if cleanup_query.get(entity) == Ok(true) {
            entity_commands.remove::<(
                MetalFx<F>,
                MetalFxRenderContext<F>,
                MainPassResolutionOverride,
                RayReconstructionDenoiser,
            )>();
        }
    }
}
