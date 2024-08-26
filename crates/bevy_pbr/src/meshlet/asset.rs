use bevy_asset::{
    io::{Reader, Writer},
    saver::{AssetSaver, SavedAsset},
    Asset, AssetLoader, AsyncReadExt, AsyncWriteExt, LoadContext,
};
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use bevy_tasks::block_on;
use bytemuck::{Pod, Zeroable};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use std::{
    io::{Read, Write},
    sync::Arc,
};

/// Unique identifier for the [`MeshletMesh`] asset format.
const MESHLET_MESH_ASSET_MAGIC: u64 = 1717551717668;

/// The current version of the [`MeshletMesh`] asset format.
pub const MESHLET_MESH_ASSET_VERSION: u64 = 1;

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
#[derive(Asset, TypePath, Clone)]
pub struct MeshletMesh {
    /// Raw vertex data bytes for the overall mesh.
    pub(crate) vertex_data: Arc<[u8]>,
    /// Indices into `vertex_data`.
    pub(crate) vertex_ids: Arc<[u32]>,
    /// Indices into `vertex_ids`.
    pub(crate) indices: Arc<[u8]>,
    /// The list of meshlets making up this mesh.
    pub(crate) meshlets: Arc<[Meshlet]>,
    /// Spherical bounding volumes.
    pub(crate) bounding_spheres: Arc<[MeshletBoundingSpheres]>,
}

/// A single meshlet within a [`MeshletMesh`].
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Meshlet {
    /// The offset within the parent mesh's [`MeshletMesh::vertex_ids`] buffer where the indices for this meshlet begin.
    pub start_vertex_id: u32,
    /// The offset within the parent mesh's [`MeshletMesh::indices`] buffer where the indices for this meshlet begin.
    pub start_index_id: u32,
    /// The amount of vertices in this meshlet.
    pub vertex_count: u32,
    /// The amount of triangles in this meshlet.
    pub triangle_count: u32,
}

/// Bounding spheres used for culling and choosing level of detail for a [`Meshlet`].
#[derive(Copy, Clone, Pod, Zeroable)]
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
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletBoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

/// An [`AssetLoader`] and [`AssetSaver`] for `.meshlet_mesh` [`MeshletMesh`] assets.
pub struct MeshletMeshSaverLoader;

impl AssetSaver for MeshletMeshSaverLoader {
    type Asset = MeshletMesh;
    type Settings = ();
    type OutputLoader = Self;
    type Error = MeshletMeshSaveOrLoadError;

    async fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, MeshletMesh>,
        _settings: &'a (),
    ) -> Result<(), MeshletMeshSaveOrLoadError> {
        // Write asset magic number
        writer
            .write_all(&MESHLET_MESH_ASSET_MAGIC.to_le_bytes())
            .await?;

        // Write asset version
        writer
            .write_all(&MESHLET_MESH_ASSET_VERSION.to_le_bytes())
            .await?;

        // Compress and write asset data
        let mut writer = FrameEncoder::new(AsyncWriteSyncAdapter(writer));
        write_slice(&asset.vertex_data, &mut writer)?;
        write_slice(&asset.vertex_ids, &mut writer)?;
        write_slice(&asset.indices, &mut writer)?;
        write_slice(&asset.meshlets, &mut writer)?;
        write_slice(&asset.bounding_spheres, &mut writer)?;
        writer.finish()?;

        Ok(())
    }
}

impl AssetLoader for MeshletMeshSaverLoader {
    type Asset = MeshletMesh;
    type Settings = ();
    type Error = MeshletMeshSaveOrLoadError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut dyn Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<MeshletMesh, MeshletMeshSaveOrLoadError> {
        // Load and check magic number
        let magic = async_read_u64(reader).await?;
        if magic != MESHLET_MESH_ASSET_MAGIC {
            return Err(MeshletMeshSaveOrLoadError::WrongFileType);
        }

        // Load and check asset version
        let version = async_read_u64(reader).await?;
        if version != MESHLET_MESH_ASSET_VERSION {
            return Err(MeshletMeshSaveOrLoadError::WrongVersion { found: version });
        }

        // Load and decompress asset data
        let reader = &mut FrameDecoder::new(AsyncReadSyncAdapter(reader));
        let vertex_data = read_slice(reader)?;
        let vertex_ids = read_slice(reader)?;
        let indices = read_slice(reader)?;
        let meshlets = read_slice(reader)?;
        let bounding_spheres = read_slice(reader)?;

        Ok(MeshletMesh {
            vertex_data,
            vertex_ids,
            indices,
            meshlets,
            bounding_spheres,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["meshlet_mesh"]
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MeshletMeshSaveOrLoadError {
    #[error("file was not a MeshletMesh asset")]
    WrongFileType,
    #[error("expected asset version {MESHLET_MESH_ASSET_VERSION} but found version {found}")]
    WrongVersion { found: u64 },
    #[error("failed to compress or decompress asset data")]
    CompressionOrDecompression(#[from] lz4_flex::frame::Error),
    #[error("failed to read or write asset data")]
    Io(#[from] std::io::Error),
}

async fn async_read_u64(reader: &mut dyn Reader) -> Result<u64, std::io::Error> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes).await?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_u64(reader: &mut dyn Read) -> Result<u64, std::io::Error> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn write_slice<T: Pod>(
    field: &[T],
    writer: &mut dyn Write,
) -> Result<(), MeshletMeshSaveOrLoadError> {
    writer.write_all(&(field.len() as u64).to_le_bytes())?;
    writer.write_all(bytemuck::cast_slice(field))?;
    Ok(())
}

fn read_slice<T: Pod>(reader: &mut dyn Read) -> Result<Arc<[T]>, std::io::Error> {
    let len = read_u64(reader)? as usize;

    let mut data: Arc<[T]> = std::iter::repeat_with(T::zeroed).take(len).collect();
    let slice = Arc::get_mut(&mut data).unwrap();
    reader.read_exact(bytemuck::cast_slice_mut(slice))?;

    Ok(data)
}

// TODO: Use async for everything and get rid of this adapter
struct AsyncWriteSyncAdapter<'a>(&'a mut Writer);

impl Write for AsyncWriteSyncAdapter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        block_on(self.0.write(buf))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        block_on(self.0.flush())
    }
}

// TODO: Use async for everything and get rid of this adapter
struct AsyncReadSyncAdapter<'a>(&'a mut dyn Reader);

impl Read for AsyncReadSyncAdapter<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        block_on(self.0.read(buf))
    }
}
