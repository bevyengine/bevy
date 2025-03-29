use bevy_asset::AssetId;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::resource::Resource;
use bevy_render::{mesh::Mesh, sync_world::MainEntityHashMap};

use super::{bind_group::Material2dBindGroupId, mesh::Mesh2dTransforms};

pub struct RenderMesh2dInstance {
    pub transforms: Mesh2dTransforms,
    pub mesh_asset_id: AssetId<Mesh>,
    pub material_bind_group_id: Material2dBindGroupId,
    pub automatic_batching: bool,
    pub tag: u32,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct RenderMesh2dInstances(MainEntityHashMap<RenderMesh2dInstance>);
