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
#![allow(clippy::type_complexity)]

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

use bevy_app::{prelude::*, MainScheduleOrder};
use bevy_ecs::schedule::ScheduleLabel;
use bevy_utils::Duration;

/// Asset storages are updated.
#[derive(Debug, Hash, PartialEq, Eq, Clone, ScheduleLabel)]
pub struct LoadAssets;
/// Asset events are generated.
#[derive(Debug, Hash, PartialEq, Eq, Clone, ScheduleLabel)]
pub struct AssetEvents;

/// Configuration for hot reloading assets by watching for changes.
#[derive(Debug, Clone)]
pub struct ChangeWatcher {
    /// Minimum delay after which a file change will trigger a reload.
    ///
    /// The change watcher will wait for this duration after a file change before reloading the
    /// asset. This is useful to avoid reloading an asset multiple times when it is changed
    /// multiple times in a short period of time, or to avoid reloading an asset that is still
    /// being written to.
    ///
    /// If you have a slow hard drive or expect to reload large assets, you may want to increase
    /// this value.
    pub delay: Duration,
}

impl ChangeWatcher {
    /// Enable change watching with the given delay when a file is changed.
    ///
    /// See [`Self::delay`] for more details on how this value is used.
    pub fn with_delay(delay: Duration) -> Option<Self> {
        Some(Self { delay })
    }
}

/// Adds support for [`Assets`] to an App.
///
/// Assets are typed collections with change tracking, which are added as App Resources. Examples of
/// assets: textures, sounds, 3d models, maps, scenes
#[derive(Debug, Clone)]
pub struct AssetPlugin {
    /// The base folder where assets are loaded from, relative to the executable.
    pub asset_folder: String,
    /// Whether to watch for changes in asset files. Requires the `filesystem_watcher` feature,
    /// and cannot be supported on the wasm32 arch nor android os.
    pub watch_for_changes: Option<ChangeWatcher>,
}

impl Default for AssetPlugin {
    fn default() -> Self {
        Self {
            asset_folder: "assets".to_string(),
            watch_for_changes: None,
        }
    }
}

impl AssetPlugin {
    /// Creates an instance of the platform's default [`AssetIo`].
    ///
    /// This is useful when providing a custom `AssetIo` instance that needs to
    /// delegate to the default `AssetIo` for the platform.
    pub fn create_platform_default_asset_io(&self) -> Box<dyn AssetIo> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        let source = FileAssetIo::new(&self.asset_folder, &self.watch_for_changes);
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

        app.register_type::<HandleId>();
        app.register_type::<AssetPath>();

        app.add_systems(PreUpdate, asset_server::free_unused_assets_system);
        app.init_schedule(LoadAssets);
        app.init_schedule(AssetEvents);

        #[cfg(all(
            feature = "filesystem_watcher",
            all(not(target_arch = "wasm32"), not(target_os = "android"))
        ))]
        app.add_systems(LoadAssets, io::filesystem_watcher_system);

        let mut order = app.world.resource_mut::<MainScheduleOrder>();
        order.insert_after(First, LoadAssets);
        order.insert_after(PostUpdate, AssetEvents);
    }
}
