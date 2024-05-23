//! Implements loader for a Gzip compressed asset.

use bevy::{
    asset::{
        io::{Reader, VecReader},
        AssetLoader, AsyncReadExt, ErasedLoadedAsset, LoadContext, LoadDirectError,
    },
    prelude::*,
    reflect::TypePath,
};
use flate2::read::GzDecoder;
use std::io::prelude::*;
use std::marker::PhantomData;
use thiserror::Error;

#[derive(Asset, TypePath)]
struct GzAsset {
    uncompressed: ErasedLoadedAsset,
}

#[derive(Default)]
struct GzAssetLoader;

/// Possible errors that can be produced by [`GzAssetLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
enum GzAssetLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    /// An error caused when the asset path cannot be used to determine the uncompressed asset type.
    #[error("Could not determine file path of uncompressed asset")]
    IndeterminateFilePath,
    /// An error caused by the internal asset loader.
    #[error("Could not load contained asset: {0}")]
    LoadDirectError(#[from] LoadDirectError),
}

impl AssetLoader for GzAssetLoader {
    type Asset = GzAsset;
    type Settings = ();
    type Error = GzAssetLoaderError;
    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let compressed_path = load_context.path();
        let file_name = compressed_path
            .file_name()
            .ok_or(GzAssetLoaderError::IndeterminateFilePath)?
            .to_string_lossy();
        let uncompressed_file_name = file_name
            .strip_suffix(".gz")
            .ok_or(GzAssetLoaderError::IndeterminateFilePath)?;
        let contained_path = compressed_path.join(uncompressed_file_name);

        let mut bytes_compressed = Vec::new();

        reader.read_to_end(&mut bytes_compressed).await?;

        let mut decoder = GzDecoder::new(bytes_compressed.as_slice());

        let mut bytes_uncompressed = Vec::new();

        decoder.read_to_end(&mut bytes_uncompressed)?;

        // Now that we have decompressed the asset, let's pass it back to the
        // context to continue loading

        let mut reader = VecReader::new(bytes_uncompressed);

        let uncompressed = load_context
            .loader()
            .direct()
            .with_reader(&mut reader)
            .untyped()
            .load(contained_path)
            .await?;

        Ok(GzAsset { uncompressed })
    }

    fn extensions(&self) -> &[&str] {
        &["gz"]
    }
}

#[derive(Component, Default)]
struct Compressed<T> {
    compressed: Handle<GzAsset>,
    _phantom: PhantomData<T>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_asset::<GzAsset>()
        .init_asset_loader::<GzAssetLoader>()
        .add_systems(Startup, setup)
        .add_systems(Update, decompress::<Image>)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        Compressed::<Image> {
            compressed: asset_server.load("data/compressed_image.png.gz"),
            ..default()
        },
        Sprite::default(),
        TransformBundle::default(),
        VisibilityBundle::default(),
    ));
}

fn decompress<A: Asset>(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut compressed_assets: ResMut<Assets<GzAsset>>,
    query: Query<(Entity, &Compressed<A>)>,
) {
    for (entity, Compressed { compressed, .. }) in query.iter() {
        let Some(GzAsset { uncompressed }) = compressed_assets.remove(compressed) else {
            continue;
        };

        let uncompressed = uncompressed.take::<A>().unwrap();

        commands
            .entity(entity)
            .remove::<Compressed<A>>()
            .insert(asset_server.add(uncompressed));
    }
}
