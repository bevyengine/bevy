#[allow(clippy::module_inception)]
mod mesh;
/// Generation for some primitive shape meshes.
pub mod shape;

use async_trait::async_trait;
pub use mesh::*;

use crate::render_asset::RenderAssetPlugin;
use bevy_app::{App, Plugin};
use bevy_asset::AddAsset;

/// Adds the [`Mesh`] as an asset and makes sure that they are extracted and prepared for the GPU.
pub struct MeshPlugin;

#[async_trait]
impl Plugin for MeshPlugin {
    async fn build(&self, app: &mut App) {
        app.add_asset::<Mesh>()
            .add_asset::<skinning::SkinnedMeshInverseBindposes>()
            .register_type::<skinning::SkinnedMesh>()
            .add_plugin(RenderAssetPlugin::<Mesh>::default())
            .await;
    }
}
