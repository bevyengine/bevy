use bevy_asset::{
    io::{Reader, Writer},
    saver::{AssetSaver, SavedAsset},
    Asset, AssetLoader, AsyncReadExt, AsyncWriteExt, LoadContext,
};
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use bevy_utils::BoxedFuture;
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A mesh that has been pre-processed into multiple small clusters of triangles called meshlets.
///
/// A [`bevy_render::mesh::Mesh`] can be converted to a [`MeshletMesh`] using `MeshletMesh::from_mesh` when the `meshlet_processor` cargo feature is enabled.
/// The conversion step is very slow, and is meant to be ran once ahead of time, and not during runtime. This type of mesh is not suitable for
/// dynamically generated geometry.
///
/// There are restrictions on the [`crate::Material`] functionality that can be used with this type of mesh.
/// * Materials have no control over the vertex shader or vertex attributes, and can only have a custom fragment shader.
/// * Materials must be opaque. Transparent, alpha masked, and transmissive materials are not supported.
/// * Materials must use the [`crate::Material::meshlet_mesh_fragment_shader`] method (and similar variants for prepass/deferred shaders)
///   which requires certain shader patterns that differ from the regular material shaders.
/// * Limited control over [`bevy_render::render_resource::RenderPipelineDescriptor`] attributes.
///
/// See also [`super::MaterialMeshletMeshBundle`].
#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct MeshletMesh {
    /// The total amount of indices in the overall mesh (sum of meshlet triangle_counts * 3).
    pub total_meshlet_indices: u64,
    /// Raw vertex data bytes for the overall mesh.
    pub vertex_data: Arc<[u8]>,
    /// Indices into `vertex_data`.
    pub vertex_ids: Arc<[u32]>,
    /// Indices into `vertex_ids`.
    pub indices: Arc<[u8]>,
    /// The list of meshlets making up this mesh.
    pub meshlets: Arc<[Meshlet]>,
    /// A list of spherical bounding volumes, 1 per meshlet.
    pub meshlet_bounding_spheres: Arc<[MeshletBoundingSphere]>,
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

/// A spherical bounding volume used for culling a [`Meshlet`].
#[derive(Serialize, Deserialize, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletBoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

/// An [`AssetLoader`] and [`AssetSaver`] for `.meshlet_mesh` [`MeshletMesh`] assets.
pub struct MeshletMeshSaverLoad;

impl AssetLoader for MeshletMeshSaverLoad {
    type Asset = MeshletMesh;
    type Settings = ();
    type Error = bincode::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            bincode::deserialize(&bytes)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["meshlet_mesh"]
    }
}

impl AssetSaver for MeshletMeshSaverLoad {
    type Asset = MeshletMesh;
    type Settings = ();
    type OutputLoader = Self;
    type Error = bincode::Error;

    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, Self::Asset>,
        _settings: &'a Self::Settings,
    ) -> BoxedFuture<'a, Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error>> {
        Box::pin(async move {
            let bytes = bincode::serialize(asset.get())?;
            writer.write_all(&bytes).await?;
            Ok(())
        })
    }
}
