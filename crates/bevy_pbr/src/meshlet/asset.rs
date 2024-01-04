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

#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct MeshletMesh {
    pub vertex_data: Arc<[u8]>,
    pub vertex_ids: Arc<[u32]>,
    pub indices: Arc<[u8]>,
    pub meshlets: Arc<[Meshlet]>,
    pub meshlet_bounding_spheres: Arc<[MeshletBoundingSphere]>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Meshlet {
    pub start_vertex_id: u32,
    pub start_index_id: u32,
    pub index_count: u32,
}

#[derive(Serialize, Deserialize, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletBoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

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
