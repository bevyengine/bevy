mod component_registry;
mod scene;
pub use component_registry::*;
pub use scene::*;

use bevy_app::{AppBuilder, AppPlugin};
use bevy_asset::AddAsset;

#[derive(Default)]
pub struct ComponentRegistryPlugin;

impl AppPlugin for ComponentRegistryPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<ComponentRegistryContext>();
    }
}

#[derive(Default)]
pub struct ScenePlugin;

impl AppPlugin for ScenePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<Scene>()
            .add_asset_loader::<Scene, SceneLoader>();
    }
}
