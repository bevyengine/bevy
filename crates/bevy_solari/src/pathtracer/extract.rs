use super::{prepare::PathtracerAccumulationTexture, Pathtracer};
use bevy_ecs::{
    change_detection::DetectChanges,
    system::{Commands, Query},
    world::Ref,
};
use bevy_render::{camera::Camera, sync_world::RenderEntity, Extract};
use bevy_transform::components::GlobalTransform;

pub fn extract_pathtracer(
    cameras_3d: Extract<
        Query<(
            RenderEntity,
            &Camera,
            Ref<GlobalTransform>,
            Option<&Pathtracer>,
        )>,
    >,
    mut commands: Commands,
) {
    for (entity, camera, global_transform, pathtracer) in &cameras_3d {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if pathtracer.is_some() && camera.is_active && camera.hdr {
            let mut pathtracer: Pathtracer = pathtracer.unwrap().clone();
            pathtracer.reset |= global_transform.is_changed();
            entity_commands.insert(pathtracer);
        } else {
            entity_commands.remove::<(Pathtracer, PathtracerAccumulationTexture)>();
        }
    }
}
