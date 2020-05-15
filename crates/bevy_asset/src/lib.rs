mod assets;
mod handle;
mod loader;
mod asset_server;
mod load_request;
mod asset_path;

pub use assets::*;
pub use handle::*;
pub use loader::*;
pub use asset_server::*;
pub use load_request::*;
pub use asset_path::*;

use bevy_app::{AppBuilder, AppPlugin};

pub mod stage {
    pub const LOAD_ASSETS: &str = "load_assets";
}

#[derive(Default)]
pub struct AssetPlugin;

impl AppPlugin for AssetPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage(stage::LOAD_ASSETS).init_resource::<AssetServer>();
    }
}
