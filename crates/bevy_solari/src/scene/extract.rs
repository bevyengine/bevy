use super::RaytracingMesh3d;
use bevy_ecs::system::{Commands, Query};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_render::{sync_world::RenderEntity, Extract};
use bevy_transform::components::GlobalTransform;

pub fn extract_raytracing_scene(
    instances: Extract<
        Query<(
            RenderEntity,
            &RaytracingMesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &GlobalTransform,
        )>,
    >,
    mut commands: Commands,
) {
    for (render_entity, mesh, material, transform) in &instances {
        commands
            .entity(render_entity)
            .insert((mesh.clone(), material.clone(), transform.clone()));
    }
}
