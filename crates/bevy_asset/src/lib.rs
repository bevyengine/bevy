mod asset_server;
mod assets;
#[cfg(feature = "filesystem_watcher")]
mod filesystem_watcher;
mod handle;
mod load_request;
mod loader;

pub use asset_server::*;
pub use assets::*;
use bevy_tasks::IoTaskPool;
pub use handle::*;
pub use load_request::*;
pub use loader::*;

/// The names of asset stages in an App Schedule
pub mod stage {
    pub const LOAD_ASSETS: &str = "load_assets";
    pub const ASSET_EVENTS: &str = "asset_events";
}

pub mod prelude {
    pub use crate::{AddAsset, AssetEvent, AssetServer, Assets, Handle};
}

use bevy_app::{prelude::Plugin, AppBuilder};
use bevy_ecs::IntoQuerySystem;
use bevy_type_registry::RegisterType;

/// Adds support for Assets to an App. Assets are typed collections with change tracking, which are added as App Resources.
/// Examples of assets: textures, sounds, 3d models, maps, scenes
#[derive(Default)]
pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let task_pool = app
            .resources()
            .get::<IoTaskPool>()
            .expect("IoTaskPool resource not found")
            .0
            .clone();
        app.add_stage_before(bevy_app::stage::PRE_UPDATE, stage::LOAD_ASSETS)
            .add_stage_after(bevy_app::stage::POST_UPDATE, stage::ASSET_EVENTS)
            .add_resource(AssetServer::new(task_pool))
            .register_property::<HandleId>();

        #[cfg(feature = "filesystem_watcher")]
        app.add_system_to_stage(
            stage::LOAD_ASSETS,
            asset_server::filesystem_watcher_system.system(),
        );
    }
}
