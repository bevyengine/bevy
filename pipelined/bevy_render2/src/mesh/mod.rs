#[allow(clippy::module_inception)]
mod mesh;
/// Generation for some primitive shape meshes.
pub mod shape;

pub use mesh::*;

use crate::render_asset::RenderAssetPlugin;
use bevy_app::{App, Plugin};
use bevy_asset::AddAsset;

pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Mesh>()
            .add_plugin(RenderAssetPlugin::<Mesh>::default());
    }
}
