use bevy_hierarchy::Children;
pub use bevy_mesh::*;
use morph::{MeshMorphWeights, MorphWeights};
pub mod allocator;
mod components;
use crate::{
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_resource::TextureView,
    texture::GpuImage,
    RenderApp,
};
use allocator::MeshAllocatorPlugin;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{AssetApp, RenderAssetUsages};
use bevy_ecs::{
    entity::Entity,
    query::{Changed, With},
    system::Query,
};
use bevy_ecs::{
    query::Without,
    system::{
        lifetimeless::{SRes, SResMut},
        SystemParamItem,
    },
};
use bitflags::bitflags;
pub use components::{Mesh2d, Mesh3d};
use wgpu::IndexFormat;

/// Adds the [`Mesh`] as an asset and makes sure that they are extracted and prepared for the GPU.
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Mesh>()
            .init_asset::<skinning::SkinnedMeshInverseBindposes>()
            .register_asset_reflect::<Mesh>()
            .register_type::<Mesh3d>()
            .register_type::<skinning::SkinnedMesh>()
            .register_type::<Vec<Entity>>()
            // 'Mesh' must be prepared after 'Image' as meshes rely on the morph target image being ready
            .add_plugins(RenderAssetPlugin::<RenderMesh, GpuImage>::default())
            .add_plugins(MeshAllocatorPlugin);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<MeshVertexBufferLayouts>();
    }
}

/// [Inherit weights](inherit_weights) from glTF mesh parent entity to direct
/// bevy mesh child entities (ie: glTF primitive).
pub struct MorphPlugin;
impl Plugin for MorphPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MorphWeights>()
            .register_type::<MeshMorphWeights>()
            .add_systems(PostUpdate, inherit_weights);
    }
}

/// Bevy meshes are gltf primitives, [`MorphWeights`] on the bevy node entity
/// should be inherited by children meshes.
///
/// Only direct children are updated, to fulfill the expectations of glTF spec.
pub fn inherit_weights(
    morph_nodes: Query<(&Children, &MorphWeights), (Without<Mesh3d>, Changed<MorphWeights>)>,
    mut morph_primitives: Query<&mut MeshMorphWeights, With<Mesh3d>>,
) {
    for (children, parent_weights) in &morph_nodes {
        let mut iter = morph_primitives.iter_many_mut(children);
        while let Some(mut child_weight) = iter.fetch_next() {
            child_weight.clear_weights();
            child_weight.extend_weights(parent_weights.weights());
        }
    }
}

bitflags! {
    /// Our base mesh pipeline key bits start from the highest bit and go
    /// downward. The PBR mesh pipeline key bits start from the lowest bit and
    /// go upward. This allows the PBR bits in the downstream crate `bevy_pbr`
    /// to coexist in the same field without any shifts.
    #[derive(Clone, Debug)]
    pub struct BaseMeshPipelineKey: u64 {
        const MORPH_TARGETS = 1 << (u64::BITS - 1);
    }
}

impl BaseMeshPipelineKey {
    pub const PRIMITIVE_TOPOLOGY_MASK_BITS: u64 = 0b111;
    pub const PRIMITIVE_TOPOLOGY_SHIFT_BITS: u64 =
        (u64::BITS - 1 - Self::PRIMITIVE_TOPOLOGY_MASK_BITS.count_ones()) as u64;

    pub fn from_primitive_topology(primitive_topology: PrimitiveTopology) -> Self {
        let primitive_topology_bits = ((primitive_topology as u64)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        Self::from_bits_retain(primitive_topology_bits)
    }

    pub fn primitive_topology(&self) -> PrimitiveTopology {
        let primitive_topology_bits = (self.bits() >> Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS;
        match primitive_topology_bits {
            x if x == PrimitiveTopology::PointList as u64 => PrimitiveTopology::PointList,
            x if x == PrimitiveTopology::LineList as u64 => PrimitiveTopology::LineList,
            x if x == PrimitiveTopology::LineStrip as u64 => PrimitiveTopology::LineStrip,
            x if x == PrimitiveTopology::TriangleList as u64 => PrimitiveTopology::TriangleList,
            x if x == PrimitiveTopology::TriangleStrip as u64 => PrimitiveTopology::TriangleStrip,
            _ => PrimitiveTopology::default(),
        }
    }
}

/// The render world representation of a [`Mesh`].
#[derive(Debug, Clone)]
pub struct RenderMesh {
    /// The number of vertices in the mesh.
    pub vertex_count: u32,

    /// Morph targets for the mesh, if present.
    pub morph_targets: Option<TextureView>,

    /// Information about the mesh data buffers, including whether the mesh uses
    /// indices or not.
    pub buffer_info: RenderMeshBufferInfo,

    /// Precomputed pipeline key bits for this mesh.
    pub key_bits: BaseMeshPipelineKey,

    /// A reference to the vertex buffer layout.
    ///
    /// Combined with [`RenderMesh::buffer_info`], this specifies the complete
    /// layout of the buffers associated with this mesh.
    pub layout: MeshVertexBufferLayoutRef,
}

impl RenderMesh {
    /// Returns the primitive topology of this mesh (triangles, triangle strips,
    /// etc.)
    #[inline]
    pub fn primitive_topology(&self) -> PrimitiveTopology {
        self.key_bits.primitive_topology()
    }
}

/// The index/vertex buffer info of a [`RenderMesh`].
#[derive(Debug, Clone)]
pub enum RenderMeshBufferInfo {
    Indexed {
        count: u32,
        index_format: IndexFormat,
    },
    NonIndexed,
}

impl RenderAsset for RenderMesh {
    type SourceAsset = Mesh;
    type Param = (
        SRes<RenderAssets<GpuImage>>,
        SResMut<MeshVertexBufferLayouts>,
    );

    #[inline]
    fn asset_usage(mesh: &Self::SourceAsset) -> RenderAssetUsages {
        mesh.asset_usage
    }

    fn byte_len(mesh: &Self::SourceAsset) -> Option<usize> {
        let mut vertex_size = 0;
        for attribute_data in mesh.attributes() {
            let vertex_format = attribute_data.0.format;
            vertex_size += vertex_format.get_size() as usize;
        }

        let vertex_count = mesh.count_vertices();
        let index_bytes = mesh.get_index_buffer_bytes().map(<[_]>::len).unwrap_or(0);
        Some(vertex_size * vertex_count + index_bytes)
    }

    /// Converts the extracted mesh into a [`RenderMesh`].
    fn prepare_asset(
        mesh: Self::SourceAsset,
        (images, ref mut mesh_vertex_buffer_layouts): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let morph_targets = match mesh.morph_targets() {
            Some(mt) => {
                let Some(target_image) = images.get(mt) else {
                    return Err(PrepareAssetError::RetryNextUpdate(mesh));
                };
                Some(target_image.texture_view.clone())
            }
            None => None,
        };

        let buffer_info = match mesh.indices() {
            Some(indices) => RenderMeshBufferInfo::Indexed {
                count: indices.len() as u32,
                index_format: indices.into(),
            },
            None => RenderMeshBufferInfo::NonIndexed,
        };

        let mesh_vertex_buffer_layout =
            mesh.get_mesh_vertex_buffer_layout(mesh_vertex_buffer_layouts);

        let mut key_bits = BaseMeshPipelineKey::from_primitive_topology(mesh.primitive_topology());
        key_bits.set(
            BaseMeshPipelineKey::MORPH_TARGETS,
            mesh.morph_targets().is_some(),
        );

        Ok(RenderMesh {
            vertex_count: mesh.count_vertices() as u32,
            buffer_info,
            key_bits,
            layout: mesh_vertex_buffer_layout,
            morph_targets,
        })
    }
}
