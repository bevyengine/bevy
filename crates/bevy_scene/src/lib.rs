mod component_registry;
mod scene;
mod scene_loader;
pub mod serde;

pub use component_registry::*;
pub use scene::*;
pub use scene_loader::*;

use bevy_app::{AppBuilder, AppPlugin};
use bevy_asset::AddAsset;

#[derive(Default)]
pub struct ComponentRegistryPlugin;

impl AppPlugin for ComponentRegistryPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<ComponentRegistryContext>()
            .init_resource::<PropertyTypeRegistryContext>();
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
