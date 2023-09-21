//! Implements a custom asset io loader.
//! An [`AssetReader`] is what the asset server uses to read the raw bytes of assets.
//! It does not know anything about the asset formats, only how to talk to the underlying storage.

use bevy::{
    asset::io::{
        file::FileAssetReader, AssetReader, AssetReaderError, AssetSource, AssetSourceId,
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
}

/// A plugins that registers our new asset reader
struct CustomAssetReaderPlugin;

impl Plugin for CustomAssetReaderPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSource::build()
                .with_reader(|| Box::new(CustomAssetReader(FileAssetReader::new("assets")))),
        );
    }
}

fn main() {
    App::new()
        .add_plugins((CustomAssetReaderPlugin, DefaultPlugins))
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
