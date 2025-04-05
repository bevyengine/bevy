use bevy_asset::AssetId;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::{Component, Tick},
    resource::Resource,
};
use bevy_math::{Affine3, Vec4};
use bevy_render::{
    mesh::Mesh,
    render_resource::{BindGroup, BindGroupId, ShaderType},
    sync_world::MainEntityHashMap,
};

use super::key::Mesh2dPipelineKey;

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct ViewKeyCache(MainEntityHashMap<Mesh2dPipelineKey>);

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct ViewSpecializationTicks(MainEntityHashMap<Tick>);

#[derive(Resource)]
pub struct Mesh2dBindGroup {
    pub value: BindGroup,
}

#[derive(Component)]
pub struct Mesh2dViewBindGroup {
    pub value: BindGroup,
}

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut)]
pub struct Material2dBindGroupId(pub Option<BindGroupId>);

#[derive(ShaderType, Clone, Copy)]
pub struct Mesh2dUniform {
    // Affine 4x3 matrix transposed to 3x4
    pub world_from_local: [Vec4; 3],
    // 3x3 matrix packed in mat2x4 and f32 as:
    //   [0].xyz, [1].x,
    //   [1].yz, [2].xy
    //   [2].z
    pub local_from_world_transpose_a: [Vec4; 2],
    pub local_from_world_transpose_b: f32,
    pub flags: u32,
    pub tag: u32,
}

impl Mesh2dUniform {
    pub fn from_components(mesh_transforms: &Mesh2dTransforms, tag: u32) -> Self {
        let (local_from_world_transpose_a, local_from_world_transpose_b) =
            mesh_transforms.world_from_local.inverse_transpose_3x3();
        Self {
            world_from_local: mesh_transforms.world_from_local.to_transpose(),
            local_from_world_transpose_a,
            local_from_world_transpose_b,
            flags: mesh_transforms.flags,
            tag,
        }
    }
}

pub struct RenderMesh2dInstance {
    pub transforms: Mesh2dTransforms,
    pub mesh_asset_id: AssetId<Mesh>,
    pub material_bind_group_id: Material2dBindGroupId,
    pub automatic_batching: bool,
    pub tag: u32,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct RenderMesh2dInstances(MainEntityHashMap<RenderMesh2dInstance>);

#[derive(Component)]
pub struct Mesh2dTransforms {
    pub world_from_local: Affine3,
    pub flags: u32,
}

// NOTE: These must match the bit flags in bevy_sprite/src/mesh2d/mesh2d.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct MeshFlags: u32 {
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

#[derive(Component, Default)]
pub(super) struct Mesh2dMarker;
