use bevy_asset::{
    io::{AsyncReadAndSeek, Reader, Writer},
    saver::{AssetSaver, SavedAsset},
    Asset, AssetLoader, AsyncReadExt, AsyncWriteExt, LoadContext,
};
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use bytemuck::{Pod, Zeroable};
use lz4_flex::block::DecompressError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The current version of the [`MeshletMesh`] asset format.
pub const MESHLET_MESH_ASSET_VERSION: u64 = 0;

/// A mesh that has been pre-processed into multiple small clusters of triangles called meshlets.
///
/// A [`bevy_render::mesh::Mesh`] can be converted to a [`MeshletMesh`] using `MeshletMesh::from_mesh` when the `meshlet_processor` cargo feature is enabled.
/// The conversion step is very slow, and is meant to be ran once ahead of time, and not during runtime. This type of mesh is not suitable for
/// dynamically generated geometry.
///
/// There are restrictions on the [`crate::Material`] functionality that can be used with this type of mesh.
/// * Materials have no control over the vertex shader or vertex attributes.
/// * Materials must be opaque. Transparent, alpha masked, and transmissive materials are not supported.
/// * Materials must use the [`crate::Material::meshlet_mesh_fragment_shader`] method (and similar variants for prepass/deferred shaders)
///   which requires certain shader patterns that differ from the regular material shaders.
/// * Limited control over [`bevy_render::render_resource::RenderPipelineDescriptor`] attributes.
///
/// See also [`super::MaterialMeshletMeshBundle`] and [`super::MeshletPlugin`].
#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct MeshletMesh {
    /// The total amount of triangles summed across all LOD 0 meshlets in the mesh.
    pub worst_case_meshlet_triangles: u64,
    /// Raw vertex data bytes for the overall mesh.
    pub vertex_data: Arc<[u8]>,
    /// Indices into `vertex_data`.
    pub vertex_ids: Arc<[u32]>,
    /// Indices into `vertex_ids`.
    pub indices: Arc<[u8]>,
    /// The list of meshlets making up this mesh.
    pub meshlets: Arc<[Meshlet]>,
    /// Spherical bounding volumes.
    pub bounding_spheres: Arc<[MeshletBoundingSpheres]>,
    /// Simplification errors used for choosing level of detail.
    pub lod_errors: Arc<[MeshletLodErrors]>,
}

/// A single meshlet within a [`MeshletMesh`].
#[derive(Serialize, Deserialize, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Meshlet {
    /// The offset within the parent mesh's [`MeshletMesh::vertex_ids`] buffer where the indices for this meshlet begin.
    pub start_vertex_id: u32,
    /// The offset within the parent mesh's [`MeshletMesh::indices`] buffer where the indices for this meshlet begin.
    pub start_index_id: u32,
    /// The amount of triangles in this meshlet.
    pub triangle_count: u32,
}

/// Bounding spheres used for culling and choosing level of detail for a [`Meshlet`].
#[derive(Serialize, Deserialize, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletBoundingSpheres {
    /// The bounding sphere used for frustum and occlusion culling for this meshlet.
    pub self_culling: MeshletBoundingSphere,
    /// The bounding sphere used for determining if this meshlet is at the correct level of detail for a given view.
    pub self_lod: MeshletBoundingSphere,
    /// The bounding sphere used for determining if this meshlet's parent is at the correct level of detail for a given view.
    pub parent_lod: MeshletBoundingSphere,
}

/// A spherical bounding volume used for a [`Meshlet`].
#[derive(Serialize, Deserialize, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletBoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

/// Simplification errors used for determining level of detail for a [`Meshlet`].
#[derive(Serialize, Deserialize, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletLodErrors {
    /// The simplification error used for creating this meshlet (from its children).
    pub self_: f32,
    /// The simplification error used for creating this meshlet's parent (from this meshlet, and its siblings).
    pub parent: f32,
}

/// An [`AssetLoader`] and [`AssetSaver`] for `.meshlet_mesh` [`MeshletMesh`] assets.
pub struct MeshletMeshSaverLoad;

impl AssetLoader for MeshletMeshSaverLoad {
    type Asset = MeshletMesh;
    type Settings = ();
    type Error = MeshletMeshSaveOrLoadError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let version = read_u64(reader).await?;
        if version != MESHLET_MESH_ASSET_VERSION {
            return Err(MeshletMeshSaveOrLoadError::WrongVersion { found: version });
        }

        let uncompressed_size = read_u64(reader).await? as usize;
        let compressed_size = read_u64(reader).await? as usize;

        let mut bytes = Vec::with_capacity(uncompressed_size);
        reader.read_to_end(&mut bytes).await?;

        let bytes = lz4_flex::decompress(&bytes, compressed_size)
            .map_err(|e| MeshletMeshSaveOrLoadError::Decompression(e))?;

        let asset = bincode::deserialize(&bytes)?;
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["meshlet_mesh"]
    }
}

impl AssetSaver for MeshletMeshSaverLoad {
    type Asset = MeshletMesh;
    type Settings = ();
    type OutputLoader = Self;
    type Error = MeshletMeshSaveOrLoadError;

    async fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, Self::Asset>,
        _settings: &'a Self::Settings,
    ) -> Result<(), Self::Error> {
        let bytes = bincode::serialize(asset.get())?;
        let uncompressed_size = bytes.len() as u64;
        let bytes = lz4_flex::compress(&bytes);
        let compressed_size = bytes.len() as u64;

        writer
            .write_all(&MESHLET_MESH_ASSET_VERSION.to_le_bytes())
            .await?;
        writer.write_all(&uncompressed_size.to_le_bytes()).await?;
        writer.write_all(&compressed_size.to_le_bytes()).await?;
        writer.write_all(&bytes).await?;
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MeshletMeshSaveOrLoadError {
    #[error("expected version {MESHLET_MESH_ASSET_VERSION} but found version {found}")]
    WrongVersion { found: u64 },
    #[error("failed to serialize or deserialize asset data")]
    SerializationOrDeserialization(#[from] bincode::Error),
    #[error("failed to decompress asset data")]
    Decompression(DecompressError),
    #[error("failed to read or write asset data")]
    Io(#[from] std::io::Error),
}

async fn read_u64(
    reader: &mut (dyn AsyncReadAndSeek + Sync + Send + Unpin),
) -> Result<u64, bincode::Error> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes).await?;
    Ok(u64::from_le_bytes(bytes))
}
