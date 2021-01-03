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

pub use asset_server::*;
pub use assets::*;
use bevy_ecs::{IntoSystem, SystemStage};
use bevy_reflect::RegisterTypeBuilder;
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

/// Adds support for Assets to an App. Assets are typed collections with change tracking, which are added as App Resources.
/// Examples of assets: textures, sounds, 3d models, maps, scenes
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
        .resources_mut()
        .get_or_insert_with(AssetServerSettings::default);

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
        if app.resources().get::<AssetServer>().is_none() {
            let task_pool = app
                .resources()
                .get::<IoTaskPool>()
                .expect("`IoTaskPool` resource not found.")
                .0
                .clone();

            let source = create_platform_default_asset_io(app);

            let asset_server = AssetServer::with_boxed_io(source, task_pool);

            app.add_resource(asset_server);
        }

        app.add_stage_before(
            bevy_app::stage::PRE_UPDATE,
            stage::LOAD_ASSETS,
            SystemStage::parallel(),
        )
        .add_stage_after(
            bevy_app::stage::POST_UPDATE,
            stage::ASSET_EVENTS,
            SystemStage::parallel(),
        )
        .register_type::<HandleId>()
        .add_system_to_stage(
            bevy_app::stage::PRE_UPDATE,
            asset_server::free_unused_assets_system.system(),
        );

        #[cfg(all(
            feature = "filesystem_watcher",
            all(not(target_arch = "wasm32"), not(target_os = "android"))
        ))]
        app.add_system_to_stage(stage::LOAD_ASSETS, io::filesystem_watcher_system.system());
    }
}
