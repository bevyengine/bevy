use alloc::sync::Arc;
use bevy_asset::{
    io::{Reader, Writer},
    saver::{AssetSaver, SavedAsset},
    Asset, AssetLoader, AsyncReadExt, AsyncWriteExt, LoadContext,
};
use bevy_math::{Vec2, Vec3};
use bevy_reflect::TypePath;
use bevy_tasks::block_on;
use bytemuck::{Pod, Zeroable};
use half::f16;
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use std::io::{Read, Write};
use thiserror::Error;

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
/// * Do not use normal maps baked from higher-poly geometry. Use the high-poly geometry directly and skip the normal map.
///   * If additional detail is needed, a smaller tiling normal map not baked from a mesh is ok.
/// * Material shaders must not use builtin functions that automatically calculate derivatives <https://gpuweb.github.io/gpuweb/wgsl/#derivatives>.
///   * Performing manual arithmetic on texture coordinates (UVs) is forbidden. Use the chain-rule version of arithmetic functions instead (TODO: not yet implemented).
/// * Limited control over [`bevy_render::render_resource::RenderPipelineDescriptor`] attributes.
/// * Materials must use the [`crate::Material::meshlet_mesh_fragment_shader`] method (and similar variants for prepass/deferred shaders)
///   which requires certain shader patterns that differ from the regular material shaders.
///
/// See also [`super::MeshletMesh3d`] and [`super::MeshletPlugin`].
#[derive(Asset, TypePath, Clone)]
pub struct MeshletMesh {
    /// Quantized and bitstream-packed vertex positions for meshlet vertices.
    pub(crate) vertex_positions: Arc<[u32]>,
    /// Octahedral-encoded and 2x16snorm packed normals for meshlet vertices.
    pub(crate) vertex_normals: Arc<[u32]>,
    /// Uncompressed vertex texture coordinates for meshlet vertices.
    pub(crate) vertex_uvs: Arc<[Vec2]>,
    /// Triangle indices for meshlets.
    pub(crate) indices: Arc<[u8]>,
    /// The list of meshlets making up this mesh.
    pub(crate) meshlets: Arc<[Meshlet]>,
    /// Spherical bounding volumes.
    pub(crate) meshlet_bounding_spheres: Arc<[MeshletBoundingSpheres]>,
    /// Meshlet group and parent group simplification errors.
    pub(crate) meshlet_simplification_errors: Arc<[MeshletSimplificationError]>,
}

/// A single meshlet within a [`MeshletMesh`].
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Meshlet {
    /// The bit offset within the parent mesh's [`MeshletMesh::vertex_positions`] buffer where the vertex positions for this meshlet begin.
    pub start_vertex_position_bit: u32,
    /// The offset within the parent mesh's [`MeshletMesh::vertex_normals`] and [`MeshletMesh::vertex_uvs`] buffers
    /// where non-position vertex attributes for this meshlet begin.
    pub start_vertex_attribute_id: u32,
    /// The offset within the parent mesh's [`MeshletMesh::indices`] buffer where the indices for this meshlet begin.
    pub start_index_id: u32,
    /// The amount of vertices in this meshlet.
    pub vertex_count: u8,
    /// The amount of triangles in this meshlet.
    pub triangle_count: u8,
    /// Unused.
    pub padding: u16,
    /// Number of bits used to store the X channel of vertex positions within this meshlet.
    pub bits_per_vertex_position_channel_x: u8,
    /// Number of bits used to store the Y channel of vertex positions within this meshlet.
    pub bits_per_vertex_position_channel_y: u8,
    /// Number of bits used to store the Z channel of vertex positions within this meshlet.
    pub bits_per_vertex_position_channel_z: u8,
    /// Power of 2 factor used to quantize vertex positions within this meshlet.
    pub vertex_position_quantization_factor: u8,
    /// Minimum quantized X channel value of vertex positions within this meshlet.
    pub min_vertex_position_channel_x: f32,
    /// Minimum quantized Y channel value of vertex positions within this meshlet.
    pub min_vertex_position_channel_y: f32,
    /// Minimum quantized Z channel value of vertex positions within this meshlet.
    pub min_vertex_position_channel_z: f32,
}

/// Bounding spheres used for culling and choosing level of detail for a [`Meshlet`].
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletBoundingSpheres {
    /// Bounding sphere used for frustum and occlusion culling for this meshlet.
    pub culling_sphere: MeshletBoundingSphere,
    /// Bounding sphere used for determining if this meshlet's group is at the correct level of detail for a given view.
    pub lod_group_sphere: MeshletBoundingSphere,
    /// Bounding sphere used for determining if this meshlet's parent group is at the correct level of detail for a given view.
    pub lod_parent_group_sphere: MeshletBoundingSphere,
}

/// A spherical bounding volume used for a [`Meshlet`].
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletBoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

/// Simplification error used for choosing level of detail for a [`Meshlet`].
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletSimplificationError {
    /// Simplification error used for determining if this meshlet's group is at the correct level of detail for a given view.
    pub group_error: f16,
    /// Simplification error used for determining if this meshlet's parent group is at the correct level of detail for a given view.
    pub parent_group_error: f16,
}

/// An [`AssetSaver`] for `.meshlet_mesh` [`MeshletMesh`] assets.
pub struct MeshletMeshSaver;

impl AssetSaver for MeshletMeshSaver {
    type Asset = MeshletMesh;
    type Settings = ();
    type OutputLoader = MeshletMeshLoader;
    type Error = MeshletMeshSaveOrLoadError;

    async fn save(
        &self,
        writer: &mut Writer,
        asset: SavedAsset<'_, MeshletMesh>,
        _settings: &(),
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
        write_slice(&asset.vertex_positions, &mut writer)?;
        write_slice(&asset.vertex_normals, &mut writer)?;
        write_slice(&asset.vertex_uvs, &mut writer)?;
        write_slice(&asset.indices, &mut writer)?;
        write_slice(&asset.meshlets, &mut writer)?;
        write_slice(&asset.meshlet_bounding_spheres, &mut writer)?;
        write_slice(&asset.meshlet_simplification_errors, &mut writer)?;
        writer.finish()?;

        Ok(())
    }
}

/// An [`AssetLoader`] for `.meshlet_mesh` [`MeshletMesh`] assets.
pub struct MeshletMeshLoader;

impl AssetLoader for MeshletMeshLoader {
    type Asset = MeshletMesh;
    type Settings = ();
    type Error = MeshletMeshSaveOrLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
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
        let vertex_positions = read_slice(reader)?;
        let vertex_normals = read_slice(reader)?;
        let vertex_uvs = read_slice(reader)?;
        let indices = read_slice(reader)?;
        let meshlets = read_slice(reader)?;
        let meshlet_bounding_spheres = read_slice(reader)?;
        let meshlet_simplification_errors = read_slice(reader)?;

        Ok(MeshletMesh {
            vertex_positions,
            vertex_normals,
            vertex_uvs,
            indices,
            meshlets,
            meshlet_bounding_spheres,
            meshlet_simplification_errors,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["meshlet_mesh"]
    }
}

#[derive(Error, Debug)]
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

    let mut data: Arc<[T]> = core::iter::repeat_with(T::zeroed).take(len).collect();
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
