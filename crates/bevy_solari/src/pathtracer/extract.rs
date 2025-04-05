use super::{prepare::PathtracerAccumulationTexture, Pathtracer};
use bevy_ecs::system::{Commands, Query};
use bevy_render::{camera::Camera, sync_world::RenderEntity, Extract};

pub fn extract_pathtracer(
    cameras_3d: Extract<Query<(RenderEntity, &Camera, Option<&Pathtracer>)>>,
    mut commands: Commands,
) {
    for (entity, camera, pathtracer) in &cameras_3d {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if pathtracer.is_some() && camera.is_active && camera.hdr {
            entity_commands.insert(pathtracer.as_deref().unwrap().clone());
        } else {
            entity_commands.remove::<(Pathtracer, PathtracerAccumulationTexture)>();
        }
    }
}
