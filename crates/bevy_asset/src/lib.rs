//! Built-in plugin for asset support.
//!
//! This plugin allows a bevy app to work with assets from the filesystem (or [another source]),
//! providing an [asset server] for loading and processing [`Asset`]s and storing them in an
//! [asset storage] to be accessed by systems.
//!
//! [another source]: trait.AssetIo.html
//! [asset server]: struct.AssetServer.html
//! [asset storage]: struct.Assets.html

#![warn(missing_docs)]

mod asset_server;
mod assets;
#[cfg(feature = "debug_asset_server")]
pub mod debug_asset_server;
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
mod reflect;

/// The `bevy_asset` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        AddAsset, AssetEvent, AssetPlugin, AssetServer, Assets, Handle, HandleUntyped,
    };
}

pub use anyhow::Error;
pub use asset_server::*;
pub use assets::*;
pub use bevy_utils::BoxedFuture;
pub use handle::*;
pub use info::*;
pub use io::*;
pub use loader::*;
pub use path::*;
pub use reflect::*;

use bevy_app::{prelude::Plugin, App};
use bevy_ecs::schedule::{StageLabel, SystemStage};

/// The names of asset stages in an [`App`] schedule.
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum AssetStage {
    /// The stage where asset storages are updated.
    LoadAssets,
    /// The stage where asset events are generated.
    AssetEvents,
}

/// Adds support for Assets to an App.
///
/// Assets are typed collections with change tracking, which are added as App Resources. Examples of
/// assets: textures, sounds, 3d models, maps, scenes
#[derive(Debug, Clone)]
pub struct AssetPlugin {
    /// The base folder where assets are loaded from, relative to the executable.
    pub asset_folder: String,
    /// Whether to watch for changes in asset files. Requires the `filesystem_watcher` feature,
    /// and cannot be supported on the wasm32 arch nor android os.
    pub watch_for_changes: bool,
}

impl Default for AssetPlugin {
    fn default() -> Self {
        Self {
            asset_folder: "assets".to_string(),
            watch_for_changes: false,
        }
    }
}

impl AssetPlugin {
    /// Creates an instance of the platform's default `AssetIo`.
    ///
    /// This is useful when providing a custom `AssetIo` instance that needs to
    /// delegate to the default `AssetIo` for the platform.
    pub fn create_platform_default_asset_io(&self) -> Box<dyn AssetIo> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        let source = FileAssetIo::new(&self.asset_folder, self.watch_for_changes);
        #[cfg(target_arch = "wasm32")]
        let source = WasmAssetIo::new(&self.asset_folder);
        #[cfg(target_os = "android")]
        let source = AndroidAssetIo::new(&self.asset_folder);

        Box::new(source)
    }
}

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        if !app.world.contains_resource::<AssetServer>() {
            let source = self.create_platform_default_asset_io();
            let asset_server = AssetServer::with_boxed_io(source);
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
