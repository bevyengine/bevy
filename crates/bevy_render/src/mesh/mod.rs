pub mod allocator;
#[cfg(feature = "morph")]
pub mod morph;

use crate::{
    render_asset::{AssetExtractionError, PrepareAssetError, RenderAsset, RenderAssetPlugin},
    renderer::{RenderDevice, RenderQueue},
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
pub use bevy_mesh::*;
use wgpu::IndexFormat;

#[cfg(feature = "morph")]
use crate::mesh::morph::RenderMorphTargetAllocator;

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

    fn finish(&self, app: &mut App) {
        let Some(_render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        #[cfg(feature = "morph")]
        _render_app.init_resource::<RenderMorphTargetAllocator>();
    }
}

/// The render world representation of a [`Mesh`].
#[derive(Debug, Clone)]
pub struct RenderMesh {
    /// The number of vertices in the mesh.
    pub vertex_count: u32,

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

    /// Returns true if this mesh uses an index buffer or false otherwise.
    #[inline]
    pub fn indexed(&self) -> bool {
        matches!(self.buffer_info, RenderMeshBufferInfo::Indexed { .. })
    }

    #[inline]
    pub fn has_morph_targets(&self) -> bool {
        self.key_bits.contains(BaseMeshPipelineKey::MORPH_TARGETS)
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

    #[cfg(not(feature = "morph"))]
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SResMut<MeshVertexBufferLayouts>,
        (),
    );
    #[cfg(feature = "morph")]
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SResMut<MeshVertexBufferLayouts>,
        SResMut<RenderMorphTargetAllocator>,
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
        _mesh_id: AssetId<Self::SourceAsset>,
        (
            _render_device,
            _render_queue,
            mesh_vertex_buffer_layouts,
            _render_morph_targets_allocator,
        ): &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
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

        // Place the morph displacements in an image if necessary.
        #[cfg(feature = "morph")]
        if let Some(morph_targets) = mesh.morph_targets() {
            _render_morph_targets_allocator.allocate(
                _render_device,
                _render_queue,
                _mesh_id,
                morph_targets,
                mesh.count_vertices(),
            );
        }

        Ok(RenderMesh {
            vertex_count: mesh.count_vertices() as u32,
            buffer_info,
            key_bits,
            layout: mesh_vertex_buffer_layout,
        })
    }

    fn unload_asset(
        _mesh_id: AssetId<Self::SourceAsset>,
        (_, _, _, _render_morph_targets_allocator): &mut SystemParamItem<Self::Param>,
    ) {
        // Free the morph target images if necessary.
        #[cfg(feature = "morph")]
        _render_morph_targets_allocator.free(_mesh_id);
    }
}
