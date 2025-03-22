use bevy_asset::AssetId;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::resource::Resource;
use bevy_render::{mesh::Mesh, sync_world::MainEntityHashMap};

use crate::material::rendering::Material2dBindGroupId;

use super::components::Mesh2dTransforms;

/// Information of 2d mesh instances
pub struct RenderMesh2dInstance {
    /// Transform of the [`Mesh`]
    pub transforms: Mesh2dTransforms,
    /// Id of [`Mesh`] to be used for instance
    pub mesh_asset_id: AssetId<Mesh>,
    /// [`Material2dBindGroupId`] of the material used by instance
    pub material_bind_group_id: Material2dBindGroupId,
    /// Enables automatic batching
    pub automatic_batching: bool,
    /// Tag of the instance
    pub tag: u32,
}

/// Association between [`MainEntity`](bevy_render::sync_world::MainEntity) and [`RenderMesh2dInstance`]
#[derive(Default, Resource, Deref, DerefMut)]
pub struct RenderMesh2dInstances(MainEntityHashMap<RenderMesh2dInstance>);
