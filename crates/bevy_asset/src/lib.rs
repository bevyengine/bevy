mod asset_server;
mod assets;
#[cfg(feature = "filesystem_watcher")]
mod filesystem_watcher;
mod handle;
mod info;
mod io;
mod loader;
mod path;

pub use asset_server::*;
pub use assets::*;
use bevy_tasks::IoTaskPool;
pub use handle::*;
pub use info::*;
pub use io::*;
pub use loader::*;
pub use path::*;

/// The names of asset stages in an App Schedule
pub mod stage {
    pub const LOAD_ASSETS: &str = "load_assets";
    pub const ASSET_EVENTS: &str = "asset_events";
}

pub mod prelude {
    pub use crate::{AddAsset, AssetEvent, AssetServer, Assets, Handle, HandleUntyped};
}

use bevy_app::{prelude::Plugin, AppBuilder};
use bevy_ecs::IntoQuerySystem;
use bevy_type_registry::RegisterType;

/// Adds support for Assets to an App. Assets are typed collections with change tracking, which are added as App Resources.
/// Examples of assets: textures, sounds, 3d models, maps, scenes
#[derive(Default)]
pub struct AssetPlugin;

pub struct AssetServerSettings {
    asset_folder: String,
}

impl Default for AssetServerSettings {
    fn default() -> Self {
        Self {
            asset_folder: "assets".to_string(),
        }
    }
}

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let task_pool = app
            .resources()
            .get::<IoTaskPool>()
            .expect("IoTaskPool resource not found")
            .0
            .clone();

        let asset_server = {
            let settings = app
                .resources_mut()
                .get_or_insert_with(AssetServerSettings::default);
            let source = FileAssetIo::new(&settings.asset_folder);
            AssetServer::new(source, task_pool)
        };

        app.add_stage_before(bevy_app::stage::PRE_UPDATE, stage::LOAD_ASSETS)
            .add_stage_after(bevy_app::stage::POST_UPDATE, stage::ASSET_EVENTS)
            .add_resource(asset_server)
            .register_property::<HandleId>()
            .add_system_to_stage(
                bevy_app::stage::PRE_UPDATE,
                asset_server::free_unused_assets_system.system(),
            );

        #[cfg(feature = "filesystem_watcher")]
        app.add_system_to_stage(stage::LOAD_ASSETS, io::filesystem_watcher_system.system());
    }
}
