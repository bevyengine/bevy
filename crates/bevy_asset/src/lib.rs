mod asset_server;
mod assets;
mod handle;
mod load_request;
mod loader;
#[cfg(feature = "filesystem_watcher")]
pub mod filesystem_watcher;

pub use asset_server::*;
pub use assets::*;
pub use handle::*;
pub use load_request::*;
pub use loader::*;

use bevy_app::{AppBuilder, AppPlugin};
use legion::prelude::IntoSystem;

pub mod stage {
    pub const LOAD_ASSETS: &str = "load_assets";
}

#[derive(Default)]
pub struct AssetPlugin;

impl AppPlugin for AssetPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_before(bevy_app::stage::PRE_UPDATE,  stage::LOAD_ASSETS)
            .init_resource::<AssetServer>();
        #[cfg(feature = "filesystem_watcher")]
        app.add_system_to_stage(stage::LOAD_ASSETS, AssetServer::filesystem_watcher_system.system());
    }
}