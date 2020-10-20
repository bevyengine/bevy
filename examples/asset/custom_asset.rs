use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    type_registry::TypeUuid,
    utils::BoxedFuture,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, TypeUuid)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct CustomAsset {
    pub value: i32,
}

#[derive(Default)]
pub struct CustomAssetLoader;

impl AssetLoader for CustomAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let custom_asset = ron::de::from_bytes::<CustomAsset>(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(custom_asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["custom"]
    }
}

fn main() {
    App::build()
        .add_default_plugins()
        .init_resource::<State>()
        .add_asset::<CustomAsset>()
        .init_asset_loader::<CustomAssetLoader>()
        .add_startup_system(setup.system())
        .add_system(print_on_load.system())
        .run();
}

#[derive(Default)]
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

    println!("Custom asset loaded: {:?}", custom_asset.unwrap());
    state.printed = true;
}
