mod loader;
pub use loader::*;

use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_render::mesh::Mesh;

#[derive(Default)]
pub struct GltfPlugin;

impl AppPlugin for GltfPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset_loader::<Mesh, GltfLoader>();
    }
}
