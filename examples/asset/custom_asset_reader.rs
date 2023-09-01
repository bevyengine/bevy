//! Implements a custom asset io loader.
//! An [`AssetReader`] is what the asset server uses to read the raw bytes of assets.
//! It does not know anything about the asset formats, only how to talk to the underlying storage.

use bevy::{
    asset::io::{
        file::FileAssetReader, AssetProvider, AssetProviders, AssetReader, AssetReaderError,
        PathStream, Reader,
    },
    prelude::*,
    utils::BoxedFuture,
};
use std::path::Path;

/// A custom asset reader implementation that wraps a given asset reader implementation
struct CustomAssetReader<T: AssetReader>(T);

impl<T: AssetReader> AssetReader for CustomAssetReader<T> {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        info!("Reading {:?}", path);
        self.0.read(path)
    }
    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        self.0.read_meta(path)
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        self.0.read_directory(path)
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
        self.0.is_directory(path)
    }

    fn watch_for_changes(
        &self,
        event_sender: crossbeam_channel::Sender<bevy_internal::asset::io::AssetSourceEvent>,
    ) -> Option<Box<dyn bevy_internal::asset::io::AssetWatcher>> {
        self.0.watch_for_changes(event_sender)
    }
}

/// A plugins that registers our new asset reader
struct CustomAssetReaderPlugin;

impl Plugin for CustomAssetReaderPlugin {
    fn build(&self, app: &mut App) {
        let mut asset_providers = app
            .world
            .get_resource_or_insert_with::<AssetProviders>(Default::default);
        asset_providers.insert_reader("CustomAssetReader", || {
            Box::new(CustomAssetReader(FileAssetReader::new("assets")))
        });
    }
}

fn main() {
    App::new()
        .add_plugins((
            CustomAssetReaderPlugin,
            DefaultPlugins.set(AssetPlugin::Unprocessed {
                source: AssetProvider::Custom("CustomAssetReader".to_string()),
                watch_for_changes: false,
            }),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: asset_server.load("branding/icon.png"),
        ..default()
    });
}
