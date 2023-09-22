//! Implements loader for a custom asset type.

use bevy::{
    asset::{anyhow::Error, io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::TypePath,
    utils::BoxedFuture,
};
use futures_lite::AsyncReadExt;
use serde::Deserialize;

#[derive(Asset, TypePath, Debug, Deserialize)]
pub struct CustomAsset {
    pub value: i32,
}

#[derive(Default)]
pub struct CustomAssetLoader;

impl AssetLoader for CustomAssetLoader {
    type Asset = CustomAsset;
    type Settings = ();
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let custom_asset = ron::de::from_bytes::<CustomAsset>(&bytes)?;
            Ok(custom_asset)
        })
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

fn print_on_load(mut state: ResMut<State>, custom_assets: ResMut<Assets<CustomAsset>>) {
    let custom_asset = custom_assets.get(&state.handle);
    if state.printed || custom_asset.is_none() {
        return;
    }

    info!("Custom asset loaded: {:?}", custom_asset.unwrap());
    state.printed = true;
}
