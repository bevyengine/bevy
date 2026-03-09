use core::marker::PhantomData;

use super::RaytracingMesh3d;
use bevy_asset::{AssetData, AssetEntity, AssetId};
use bevy_derive::Deref;
use bevy_ecs::{
    entity::Entity,
    lifecycle::RemovedComponents,
    query::Changed,
    resource::Resource,
    system::{Commands, Query, ResMut},
};
use bevy_pbr::{MeshMaterial3d, PreviousGlobalTransform, StandardMaterial};
use bevy_platform::collections::HashMap;
use bevy_render::{sync_world::RenderEntity, Extract};
use bevy_transform::components::GlobalTransform;

pub fn extract_raytracing_scene(
    instances: Extract<
        Query<(
            RenderEntity,
            &RaytracingMesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
        )>,
    >,
    mut commands: Commands,
) {
    for (render_entity, mesh, material, transform, previous_frame_transform) in &instances {
        let mut commands = commands.entity(render_entity);

        match previous_frame_transform.cloned() {
            Some(previous_frame_transform) => commands.insert((
                mesh.clone(),
                material.clone(),
                *transform,
                previous_frame_transform,
            )),
            None => commands.insert((mesh.clone(), material.clone(), *transform)),
        };
    }
}

#[derive(Resource, Deref, Default)]
pub struct StandardMaterialAssets(HashMap<AssetId<StandardMaterial>, StandardMaterial>);

// TODO: It would be nice for `Assets` to have an API for this instead of us needing to drop down
// into raw queries.
pub fn extract_standard_material_assets(
    mut removed_components: Extract<RemovedComponents<AssetData<StandardMaterial>>>,
    main_world_assets: Extract<
        Query<(Entity, &AssetData<StandardMaterial>), Changed<AssetData<StandardMaterial>>>,
    >,
    mut extracted_assets: ResMut<StandardMaterialAssets>,
) {
    for entity in removed_components.read() {
        extracted_assets.0.remove(&AssetId::Entity {
            entity: AssetEntity::new_unchecked(entity),
            marker: PhantomData,
        });
    }
    for (entity, asset_data) in main_world_assets.iter() {
        extracted_assets.0.insert(
            AssetId::Entity {
                entity: AssetEntity::new_unchecked(entity),
                marker: PhantomData,
            },
            asset_data.0.clone(),
        );
    }
}
