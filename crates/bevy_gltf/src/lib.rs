mod loader;
pub use loader::*;

use bevy_app::{AppPlugin, AppBuilder};
use bevy_asset::{AddAsset};

#[derive(Default)]
pub struct GltfPlugin;

impl AppPlugin for GltfPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset_loader(GltfLoader);
    }
}