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

use bevy_app::{prelude::Plugin, App};
use bevy_ecs::schedule::{StageLabel, SystemStage};
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
pub fn create_platform_default_asset_io(app: &mut App) -> Box<dyn AssetIo> {
    let settings = app
        .world
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
    fn build(&self, app: &mut App) {
        if app.world.get_resource::<AssetServer>().is_none() {
            let task_pool = app
                .world
                .get_resource::<IoTaskPool>()
                .expect("`IoTaskPool` resource not found.")
                .0
                .clone();

            let source = create_platform_default_asset_io(app);

            let asset_server = AssetServer::with_boxed_io(source, task_pool);

            app.insert_resource(asset_server);
        }

        app.add_stage_before(
            bevy_app::CoreStage::PreUpdate,
            AssetStage::LoadAssets,
            SystemStage::parallel(),
        )
        .add_stage_after(
            bevy_app::CoreStage::PostUpdate,
            AssetStage::AssetEvents,
            SystemStage::parallel(),
        )
        .register_type::<HandleId>()
        .add_system_to_stage(
            bevy_app::CoreStage::PreUpdate,
            asset_server::free_unused_assets_system,
        );

        #[cfg(all(
            feature = "filesystem_watcher",
            all(not(target_arch = "wasm32"), not(target_os = "android"))
        ))]
        app.add_system_to_stage(AssetStage::LoadAssets, io::filesystem_watcher_system);
    }
}
