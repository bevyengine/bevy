pub use asset_server::*;
pub use assets::*;
use bevy_app::{AppBuilder, prelude::Plugin};
use bevy_tasks::{ComputeTaskPool, IoTaskPool};
use bevy_type_registry::RegisterType;
pub use handle::*;
pub use info::*;
pub use io::*;
pub use loader::*;
pub use path::*;

mod asset_server;
mod assets;
pub mod diagnostic;
#[cfg(all(
feature = "filesystem_watcher",
all(not(target_arch = "wasm32"), not(target_os = "android"))
))]
mod filesystem_watcher;
mod handle;
mod info;
mod io;
mod loader;
mod path;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{AddAsset, AssetEvent, AssetServer, Assets, Handle, HandleUntyped};
}

pub use asset_server::*;
pub use assets::*;
pub use bevy_utils::BoxedFuture;
pub use handle::*;
pub use info::*;
pub use io::*;
pub use loader::*;
pub use path::*;

use bevy_app::{prelude::Plugin, AppBuilder};
use bevy_ecs::{
    schedule::{StageLabel, SystemStage},
    system::IntoSystem,
};
use bevy_tasks::IoTaskPool;

/// The names of asset stages in an App Schedule
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum AssetStage {
    LoadAssets,
    AssetEvents,
}

/// Adds support for Assets to an App. Assets are typed collections with change tracking, which are
/// added as App Resources. Examples of assets: textures, sounds, 3d models, maps, scenes
#[derive(Default)]
pub struct AssetPlugin;

pub struct AssetServerSettings {
    pub asset_folder: String,
}

impl Default for AssetServerSettings {
    fn default() -> Self {
        Self {
            asset_folder: "assets".to_string(),
        }
    }
}

/// Create an instance of the platform default `AssetIo`
///
/// This is useful when providing a custom `AssetIo` instance that needs to
/// delegate to the default `AssetIo` for the platform.
pub fn create_platform_default_asset_io(app: &mut AppBuilder) -> Box<dyn AssetIo> {
    let settings = app
        .world_mut()
        .get_resource_or_insert_with(AssetServerSettings::default);

    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    let source = FileAssetIo::new(&settings.asset_folder);
    #[cfg(target_arch = "wasm32")]
    let source = WasmAssetIo::new(&settings.asset_folder);
    #[cfg(target_os = "android")]
    let source = AndroidAssetIo::new(&settings.asset_folder);

    Box::new(source)
}

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let task_pool = app
            .resources()
            .get::<ComputeTaskPool>()
            .expect("ComputeTaskPool resource not found")
            .0
            .clone();

        let asset_server = {
            let settings = app
                .resources_mut()
                .get_or_insert_with(AssetServerSettings::default);

            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            let source = FileAssetIo::new(&settings.asset_folder);
            #[cfg(target_arch = "wasm32")]
            let source = WasmAssetIo::new(&settings.asset_folder);
            #[cfg(target_os = "android")]
            let source = AndroidAssetIo::new(&settings.asset_folder);
            AssetServer::new(source, task_pool)
        };

        app.add_stage_before(bevy_app::stage::PRE_UPDATE, stage::LOAD_ASSETS)
            .add_stage_after(bevy_app::stage::POST_UPDATE, stage::ASSET_EVENTS)
            .add_resource(asset_server)
            .register_property::<HandleId>()
            .add_system_to_stage(
                bevy_app::stage::PRE_UPDATE,
                asset_server::free_unused_assets_system,
            );

        #[cfg(all(
            feature = "filesystem_watcher",
            all(not(target_arch = "wasm32"), not(target_os = "android"))
        ))]
        app.add_system_to_stage(
            AssetStage::LoadAssets,
            io::filesystem_watcher_system.system(),
        );
    }
}
