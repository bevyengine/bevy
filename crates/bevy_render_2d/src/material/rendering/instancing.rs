use bevy_asset::AssetId;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    entity::Entity,
    query::{Changed, Or},
    removal_detection::RemovedComponents,
    resource::Resource,
    system::{Query, ResMut},
};
use bevy_render::{
    sync_world::{MainEntity, MainEntityHashMap},
    view::ViewVisibility,
    Extract,
};

use crate::prelude::{Material2d, MeshMaterial2d};

#[derive(Resource, Deref, DerefMut)]
pub struct RenderMaterial2dInstances<M: Material2d>(MainEntityHashMap<AssetId<M>>);

impl<M: Material2d> Default for RenderMaterial2dInstances<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub fn extract_mesh_materials_2d<M: Material2d>(
    mut material_instances: ResMut<RenderMaterial2dInstances<M>>,
    changed_meshes_query: Extract<
        Query<
            (Entity, &ViewVisibility, &MeshMaterial2d<M>),
            Or<(Changed<ViewVisibility>, Changed<MeshMaterial2d<M>>)>,
        >,
    >,
    mut removed_visibilities_query: Extract<RemovedComponents<ViewVisibility>>,
    mut removed_materials_query: Extract<RemovedComponents<MeshMaterial2d<M>>>,
) {
    for (entity, view_visibility, material) in &changed_meshes_query {
        if view_visibility.get() {
            add_mesh_instance(entity, material, &mut material_instances);
        } else {
            remove_mesh_instance(entity, &mut material_instances);
        }
    }

    for entity in removed_visibilities_query
        .read()
        .chain(removed_materials_query.read())
    {
        // Only queue a mesh for removal if we didn't pick it up above.
        // It's possible that a necessary component was removed and re-added in
        // the same frame.
        if !changed_meshes_query.contains(entity) {
            remove_mesh_instance(entity, &mut material_instances);
        }
    }

    // Adds or updates a mesh instance in the [`RenderMaterial2dInstances`]
    // array.
    fn add_mesh_instance<M>(
        entity: Entity,
        material: &MeshMaterial2d<M>,
        material_instances: &mut RenderMaterial2dInstances<M>,
    ) where
        M: Material2d,
    {
        material_instances.insert(entity.into(), material.id());
    }

    // Removes a mesh instance from the [`RenderMaterial2dInstances`] array.
    fn remove_mesh_instance<M>(
        entity: Entity,
        material_instances: &mut RenderMaterial2dInstances<M>,
    ) where
        M: Material2d,
    {
        material_instances.remove(&MainEntity::from(entity));
    }
}
