#[allow(clippy::module_inception)]
mod mesh;
/// Generation for some primitive shape meshes.
pub mod shape;

pub use mesh::*;

use crate::render_asset::RenderAssetPlugin;
use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;
use bevy_ecs::entity::Entity;

/// Adds the [`Mesh`] as an asset and makes sure that they are extracted and prepared for the GPU.
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Mesh>()
            .init_asset::<skinning::SkinnedMeshInverseBindposes>()
            .register_type::<skinning::SkinnedMesh>()
            .register_type::<Vec<Entity>>()
            .add_plugin(RenderAssetPlugin::<Mesh>::default());
    }
}
