mod loader;
pub use loader::*;

use bevy_app::prelude::*;
use bevy_asset::AddAsset;

/// Adds support for GLTF file loading to Apps
#[derive(Default)]
pub struct GltfPlugin;

impl Plugin for GltfPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset_loader::<GltfLoader>();
    }
}
