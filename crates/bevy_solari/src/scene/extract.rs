use super::RaytracingMesh3d;
use bevy_asset::{AssetId, Assets};
use bevy_derive::Deref;
use bevy_ecs::{
    resource::Resource,
    system::{Commands, Query},
};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_platform::collections::HashMap;
use bevy_render::{extract_resource::ExtractResource, sync_world::RenderEntity, Extract};
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
            .insert((mesh.clone(), material.clone(), *transform));
    }
}

#[derive(Resource, Deref, Default)]
pub struct StandardMaterialAssets(HashMap<AssetId<StandardMaterial>, StandardMaterial>);

impl ExtractResource for StandardMaterialAssets {
    type Source = Assets<StandardMaterial>;

    fn extract_resource(source: &Self::Source) -> Self {
        Self(
            source
                .iter()
                .map(|(asset_id, material)| (asset_id, material.clone()))
                .collect(),
        )
    }
}
