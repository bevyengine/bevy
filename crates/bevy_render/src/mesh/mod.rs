pub mod allocator;
#[cfg(feature = "morph")]
pub mod morph;

use crate::{
    mesh::allocator::{ElementClass, MeshAllocationKey, MeshAllocator, MeshSlabId},
    render_asset::{
        prepare_assets, AssetExtractionError, PrepareAssetError, RenderAsset, RenderAssetPlugin,
    },
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
    texture::GpuImage,
    Render, RenderApp, RenderSystems,
};
use allocator::MeshAllocatorPlugin;
use bevy_app::{App, Plugin};
use bevy_asset::{uuid_handle, AssetId, Assets, Handle, RenderAssetUsages};
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{SRes, SResMut},
        SystemParamItem,
    },
};
use bevy_encase_derive::ShaderType;
pub use bevy_mesh::*;
use bevy_shader::load_shader_library;
use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};
use wgpu::IndexFormat;

#[cfg(feature = "morph")]
use crate::mesh::morph::RenderMorphTargetAllocator;

/// Makes sure that [`Mesh`]es are extracted and prepared for the GPU.
/// Does *not* add the [`Mesh`] as an asset. Use [`MeshPlugin`] for that.
pub struct MeshRenderAssetPlugin;

impl Plugin for MeshRenderAssetPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "mesh_metadata_types.wgsl");

        app
            // 'Mesh' must be prepared after 'Image' as meshes rely on the morph target image being ready
            .add_plugins(RenderAssetPlugin::<RenderMesh, GpuImage>::default())
            .add_plugins(MeshAllocatorPlugin);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<MeshVertexBufferLayouts>()
            .add_systems(
                Render,
                prepare_mesh_metadata_fallback_buffer
                    .in_set(RenderSystems::PrepareAssets)
                    .after(prepare_assets::<RenderMesh>),
            );
    }

    fn finish(&self, app: &mut App) {
        let mut mesh_assets = app.world_mut().resource_mut::<Assets<Mesh>>();
        mesh_assets
            .insert(
                METADATA_PLACEHOLDER_MESH_HANDLE.id(),
                Mesh::new(PrimitiveTopology::PointList, RenderAssetUsages::all())
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_POSITION,
                        VertexAttributeValues::Float32x3(vec![[0.0; 3]]),
                    )
                    .with_inserted_indices(Indices::U16(vec![0]))
                    .compressed_mesh(MeshAttributeCompressionFlags::COMPRESS_POSITION, false),
            )
            .unwrap();

        let Some(_render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        #[cfg(feature = "morph")]
        crate::GpuResourceAppExt::init_gpu_resource::<RenderMorphTargetAllocator>(_render_app);
    }
}

/// A handle to a one-point compressed mesh, with 1 position and 1 index.
/// This is used to hold a metadata buffer in [`crate::mesh::allocator::MeshAllocator`] and used for fallback.
const METADATA_PLACEHOLDER_MESH_HANDLE: Handle<Mesh> =
    uuid_handle!("c79a00de-d4b9-45ac-8c12-0e65010b411b");

/// Fallback mesh metadata slab referenced by [`PLACEHOLDER_MESH_HANDLE`].
#[derive(Resource)]
pub struct MeshMetadataFallbackBuffer {
    pub slab_id: MeshSlabId,
    pub buffer: Buffer,
}

pub fn prepare_mesh_metadata_fallback_buffer(
    mut commands: Commands,
    mesh_allocator: Res<MeshAllocator>,
) {
    let slab_id = mesh_allocator
        .key_to_slab
        .get(&MeshAllocationKey::new(
            METADATA_PLACEHOLDER_MESH_HANDLE.id(),
            ElementClass::Metadata,
        ))
        .cloned()
        .unwrap();
    let buffer = mesh_allocator.buffer_for_slab(slab_id).unwrap().clone();
    commands.insert_resource(MeshMetadataFallbackBuffer { slab_id, buffer });
}

/// Per-mesh metadata, stored in [`crate::mesh::allocator::MeshAllocator`].
/// Currently this is used to decompress vertex.
#[derive(Default, Pod, Zeroable, Clone, Copy, Debug, ShaderType)]
#[repr(C)]
pub struct MeshMetadata {
    // AABB for decompressing positions.
    pub aabb_center: Vec3,
    pub pad1: u32,
    // AABB for decompressing positions.
    pub aabb_half_extents: Vec3,
    pub pad2: u32,
    // UV channels range for decompressing UVs coordinates.
    pub uv_channels_min_and_extents: [Vec4; 2],
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
    pub fn index_format(&self) -> Option<IndexFormat> {
        match self.buffer_info {
            RenderMeshBufferInfo::Indexed { index_format, .. } => Some(index_format),
            RenderMeshBufferInfo::NonIndexed => None,
        }
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
        let (buffer_info, index_format) = match mesh.indices() {
            Some(indices) => (
                RenderMeshBufferInfo::Indexed {
                    count: indices.len() as u32,
                    index_format: indices.into(),
                },
                Some(indices.into()),
            ),
            None => (RenderMeshBufferInfo::NonIndexed, None),
        };

        let mesh_vertex_buffer_layout =
            mesh.get_mesh_vertex_buffer_layout(mesh_vertex_buffer_layouts);

        let key_bits = BaseMeshPipelineKey::from_primitive_topology_and_strip_index(
            mesh.primitive_topology(),
            index_format,
        );
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
