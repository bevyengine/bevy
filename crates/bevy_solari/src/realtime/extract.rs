use super::{prepare::SolariLightingResources, SolariLighting};
use bevy_ecs::system::{Commands, ResMut};
use bevy_pbr::deferred::SkipDeferredLighting;
use bevy_render::{camera::Camera, sync_world::RenderEntity, MainWorld};

pub fn extract_solari_lighting(mut main_world: ResMut<MainWorld>, mut commands: Commands) {
    let mut cameras_3d = main_world.query::<(RenderEntity, &Camera, Option<&mut SolariLighting>)>();

    for (entity, camera, mut solari_lighting) in cameras_3d.iter_mut(&mut main_world) {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if solari_lighting.is_some() && camera.is_active {
            entity_commands.insert((
                solari_lighting.as_deref().unwrap().clone(),
                SkipDeferredLighting,
            ));
            solari_lighting.as_mut().unwrap().reset = false;
        } else {
            entity_commands.remove::<(
                SolariLighting,
                SolariLightingResources,
                SkipDeferredLighting,
            )>();
        }
    }
}
