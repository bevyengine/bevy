use bevy_render::mesh::Mesh;

use crate::gltf_reader::load_gltf;
use anyhow::Result;
use bevy_asset::AssetLoader;
use bevy_render::texture::Texture;
use std::path::Path;

/// Loads meshes from GLTF files into Mesh assets
///
/// NOTE: eventually this will loading into Scenes instead of Meshes
#[derive(Default)]
pub struct GltfLoader;

static GLTF_EXTENSIONS: &[&str] = &["gltf", "glb"];

impl AssetLoader<Mesh> for GltfLoader {
    fn from_bytes(&self, asset_path: &Path, bytes: Vec<u8>) -> Result<Mesh> {
        let mesh = load_gltf(asset_path, bytes)?
            .mesh
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("The GLTF file did not contain a mesh"))?;
        Ok(mesh)
    }

    fn extensions(&self) -> &[&str] {
        GLTF_EXTENSIONS
    }
}

impl AssetLoader<Texture> for GltfLoader {
    fn from_bytes(&self, asset_path: &Path, bytes: Vec<u8>) -> Result<Texture> {
        let texture = load_gltf(asset_path, bytes)?
            .texture
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("The GLTF file did not contain a texture"))?;
        Ok(texture)
    }

    fn extensions(&self) -> &[&str] {
        GLTF_EXTENSIONS
    }
}
