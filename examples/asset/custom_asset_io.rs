//! Implements a custom asset io loader.
//! An [`AssetIo`] is what the asset server uses to read the raw bytes of assets.
//! It does not know anything about the asset formats, only how to talk to the underlying storage.

use bevy::{
    asset::{AssetIo, AssetIoError, Metadata},
    prelude::*,
    utils::BoxedFuture,
};
use std::path::{Path, PathBuf};

/// A custom asset io implementation that simply defers to the platform default
/// implementation.
///
/// This can be used as a starting point for developing a useful implementation
/// that can defer to the default when needed.
struct CustomAssetIo(Box<dyn AssetIo>);

impl AssetIo for CustomAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        info!("load_path({:?})", path);
        self.0.load_path(path)
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        info!("read_directory({:?})", path);
        self.0.read_directory(path)
    }

    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError> {
        info!("watch_path_for_changes({:?})", path);
        self.0.watch_path_for_changes(path)
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        info!("watch_for_changes()");
        self.0.watch_for_changes()
    }

    fn get_metadata(&self, path: &Path) -> Result<Metadata, AssetIoError> {
        info!("get_metadata({:?})", path);
        self.0.get_metadata(path)
    }
}

/// A plugin used to execute the override of the asset io
struct CustomAssetIoPlugin;

impl Plugin for CustomAssetIoPlugin {
    fn build(&self, app: &mut App) {
        let asset_io = {
            // the platform default asset io requires a reference to the app
            // builder to find its configuration

            let default_io = bevy::asset::create_platform_default_asset_io(app);

            // create the custom asset io instance

            CustomAssetIo(default_io)
        };

        // the asset server is constructed and added the resource manager

        app.insert_resource(AssetServer::new(asset_io));
    }
}

fn main() {
    App::new()
        .add_plugins_with(DefaultPlugins, |group| {
            // the custom asset io plugin must be inserted in-between the
            // `CorePlugin' and `AssetPlugin`. It needs to be after the
            // CorePlugin, so that the IO task pool has already been constructed.
            // And it must be before the `AssetPlugin` so that the asset plugin
            // doesn't create another instance of an asset server. In general,
            // the AssetPlugin should still run so that other aspects of the
            // asset system are initialized correctly.
            group.add_before::<bevy::asset::AssetPlugin, _>(CustomAssetIoPlugin)
        })
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());
    commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("branding/icon.png"),
        ..default()
    });
}
