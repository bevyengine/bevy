//! Implements loader for a custom asset type.

use bevy::utils::thiserror;
use bevy::{
    asset::{io::Reader, ron, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::TypePath,
};
use serde::Deserialize;
use thiserror::Error;

#[derive(Asset, TypePath, Debug, Deserialize)]
pub struct CustomAsset {
    pub value: i32,
}

#[derive(Default)]
pub struct CustomAssetLoader;

/// Possible errors that can be produced by [`CustomAssetLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum CustomAssetLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON](ron) Error
    #[error("Could not parse RON: {0}")]
    RonSpannedError(#[from] ron::error::SpannedError),
}

impl AssetLoader for CustomAssetLoader {
    type Asset = CustomAsset;
    type Settings = ();
    type Error = CustomAssetLoaderError;
    async fn load(
        &self,
        reader: &mut Reader<'_>,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let custom_asset = ron::de::from_bytes::<CustomAsset>(&bytes)?;
        Ok(custom_asset)
    }

    fn extensions(&self) -> &[&str] {
        &["custom"]
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<State>()
        .init_asset::<CustomAsset>()
        .init_asset_loader::<CustomAssetLoader>()
        .add_systems(Startup, setup)
        .add_systems(Update, print_on_load)
        .run();
}

#[derive(Resource, Default)]
struct State {
    handle: Handle<CustomAsset>,
    printed: bool,
}

fn setup(mut state: ResMut<State>, asset_server: Res<AssetServer>) {
    state.handle = asset_server.load("data/asset.custom");
}

fn print_on_load(mut state: ResMut<State>, custom_assets: Res<Assets<CustomAsset>>) {
    let custom_asset = custom_assets.get(&state.handle);
    if state.printed || custom_asset.is_none() {
        return;
    }

    info!("Custom asset loaded: {:?}", custom_asset.unwrap());
    state.printed = true;
}
