pub mod allocator;
use crate::{
    render_asset::{
        AssetExtractionError, PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets,
    },
    texture::GpuImage,
    RenderApp,
};
use allocator::MeshAllocatorPlugin;
use bevy_app::{App, Plugin};
use bevy_asset::{AssetId, RenderAssetUsages};
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{SRes, SResMut},
        SystemParamItem,
    },
};
use bevy_math::bounding::{Aabb2d, Aabb3d};
#[cfg(feature = "morph")]
use bevy_mesh::morph::{MeshMorphWeights, MorphWeights};
use bevy_mesh::*;
use wgpu::IndexFormat;

/// Makes sure that [`Mesh`]es are extracted and prepared for the GPU.
/// Does *not* add the [`Mesh`] as an asset. Use [`MeshPlugin`] for that.
pub struct MeshRenderAssetPlugin;

impl Plugin for MeshRenderAssetPlugin {
    fn build(&self, app: &mut App) {
        app
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
#[cfg(feature = "morph")]
pub struct MorphPlugin;
#[cfg(feature = "morph")]
impl Plugin for MorphPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy_app::PostUpdate,
            inherit_weights.in_set(InheritWeightSystems),
        );
    }
}

/// Bevy meshes are gltf primitives, [`MorphWeights`] on the bevy node entity
/// should be inherited by children meshes.
///
/// Only direct children are updated, to fulfill the expectations of glTF spec.
#[cfg(feature = "morph")]
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

/// The render world representation of a [`Mesh`].
#[derive(Debug, Clone)]
pub struct RenderMesh {
    /// The number of vertices in the mesh.
    pub vertex_count: u32,

    /// Morph targets for the mesh, if present.
    #[cfg(feature = "morph")]
    pub morph_targets: Option<crate::render_resource::TextureView>,

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

    /// AABB used for decompressing vertex positions.
    /// None if the positions of the mesh is empty or the format isn't Float32x3.
    pub aabb: Option<Aabb3d>,
    /// UV0 range for decompressing UV0 coordinates.
    pub uv0_range: Option<Aabb2d>,
    /// UV1 range for decompressing UV1 coordinates.
    pub uv1_range: Option<Aabb2d>,
}

impl RenderMesh {
    /// Returns the primitive topology of this mesh (triangles, triangle strips,
    /// etc.)
    #[inline]
    pub fn primitive_topology(&self) -> PrimitiveTopology {
        self.key_bits.primitive_topology()
    }

    /// Returns true if this mesh uses an index buffer or false otherwise.
    #[inline]
    pub fn indexed(&self) -> bool {
        matches!(self.buffer_info, RenderMeshBufferInfo::Indexed { .. })
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

    fn take_gpu_data(
        source: &mut Self::SourceAsset,
        _previous_gpu_asset: Option<&Self>,
    ) -> Result<Self::SourceAsset, AssetExtractionError> {
        source
            .take_gpu_data()
            .map_err(|_| AssetExtractionError::AlreadyExtracted)
    }

    fn byte_len(mesh: &Self::SourceAsset) -> Option<usize> {
        let mut vertex_size = 0;
        for attribute_data in mesh.attributes() {
            let vertex_format = attribute_data.0.format;
            vertex_size += vertex_format.size() as usize;
        }

        let vertex_count = mesh.count_vertices();
        let index_bytes = mesh.get_index_buffer_bytes().map(<[_]>::len).unwrap_or(0);
        Some(vertex_size * vertex_count + index_bytes)
    }

    /// Converts the extracted mesh into a [`RenderMesh`].
    fn prepare_asset(
        mesh: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (_images, mesh_vertex_buffer_layouts): &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        #[cfg(feature = "morph")]
        let morph_targets = match mesh.morph_targets() {
            Some(mt) => {
                let Some(target_image) = _images.get(mt) else {
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

        let key_bits = BaseMeshPipelineKey::from_primitive_topology(mesh.primitive_topology());
        #[cfg(feature = "morph")]
        let key_bits = if mesh.morph_targets().is_some() {
            key_bits | BaseMeshPipelineKey::MORPH_TARGETS
        } else {
            key_bits
        };

        Ok(RenderMesh {
            vertex_count: mesh.count_vertices() as u32,
            buffer_info,
            key_bits,
            layout: mesh_vertex_buffer_layout,
            #[cfg(feature = "morph")]
            morph_targets,
            // `final_aabb` is not available in `prepare_asset` so we have to compute it.
            aabb: mesh.compute_aabb(),
            uv0_range: mesh.compute_uv_range(Mesh::ATTRIBUTE_UV_0),
            uv1_range: mesh.compute_uv_range(Mesh::ATTRIBUTE_UV_1),
        })
    }
}
