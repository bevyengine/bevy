use bevy_hierarchy::Children;
use bevy_math::Vec3;
pub use bevy_mesh::*;
use morph::{MeshMorphWeights, MorphWeights};
pub mod allocator;
mod components;
use crate::{
    primitives::Aabb,
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_resource::TextureView,
    texture::GpuImage,
    RenderApp,
};
use allocator::MeshAllocatorPlugin;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{AssetApp, AssetId, RenderAssetUsages};
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

pub trait MeshAabb {
    /// Compute the Axis-Aligned Bounding Box of the mesh vertices in model space
    ///
    /// Returns `None` if `self` doesn't have [`Mesh::ATTRIBUTE_POSITION`] of
    /// type [`VertexAttributeValues::Float32x3`], or if `self` doesn't have any vertices.
    fn compute_aabb(&self) -> Option<Aabb>;
}

impl MeshAabb for Mesh {
    fn compute_aabb(&self) -> Option<Aabb> {
        let Some(VertexAttributeValues::Float32x3(values)) =
            self.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            return None;
        };

        Aabb::enclosing(values.iter().map(|p| Vec3::from_slice(p)))
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
        _: AssetId<Self::SourceAsset>,
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
