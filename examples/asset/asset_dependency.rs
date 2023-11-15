//! This example demonstrates how to handle loading dependency assets.
use bevy::{
    asset::{AssetLoader, LoadedAsset},
    prelude::*,
    reflect::{TypePath, TypeUuid},
};
use serde::Deserialize;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_asset::<MyAsset>()
        .init_asset_loader::<MyAssetLoader>()
        .add_asset::<MyOtherAsset>()
        .init_asset_loader::<MyOtherAssetLoader>()
        .add_systems(Startup, setup)
        .add_systems(Update, print_my_asset)
        .run();
}

/// Temporarily store our main asset handle for later.

#[derive(Resource)]
struct ResMyAsset(Handle<MyAsset>);

/// Loading our assets and printing their values.

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // We only need to load the MyAsset, as the MyOtherAsset will be loaded as a dependency.
    let my_asset: Handle<MyAsset> = asset_server.load("data/test.my_asset.ron");

    commands.insert_resource(ResMyAsset(my_asset));
}

/// Printing our asset once it's loaded.

fn print_my_asset(
    my_asset: Res<ResMyAsset>,
    my_assets: Res<Assets<MyAsset>>,
    my_other_assets: Res<Assets<MyOtherAsset>>,
    mut printed: Local<bool>,
) {
    if let Some(my_asset) = my_assets.get(&my_asset.0) {
        if let Some(my_other_asset) = my_other_assets.get(&my_asset.other_asset_handle) {
            // To prevent spam in the console output.
            if !*printed {
                *printed = true;

                info!(
                    "my_asset: {}, my_other_asset: {}",
                    my_asset.value, my_other_asset.value
                );
            }
        }
    }
}

/// Our main asset that stores a handle to another asset.

#[derive(Debug, Clone, Deserialize, TypeUuid, TypePath)]
#[uuid = "2104f5b8-97f8-489d-8bc0-ff0fdf7c7c28"]
struct MyAsset {
    value: i32,
    other_asset: String,
    #[serde(skip)]
    other_asset_handle: Handle<MyOtherAsset>,
}

/// Our main asset loader.

#[derive(Default)]
struct MyAssetLoader;

impl AssetLoader for MyAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            // We load the main asset normally.
            let mut my_asset: MyAsset = ron::de::from_bytes(bytes)?;

            // Clone the path to the dependency asset (as it is moved later on).
            let other_asset_path = my_asset.other_asset.clone();

            // Obtain the asset Handle for the dependency asset path.
            let other_asset: Handle<MyOtherAsset> = load_context.get_handle(&other_asset_path);

            my_asset.other_asset_handle = other_asset;

            load_context.set_default_asset(
                LoadedAsset::new(my_asset).with_dependency(other_asset_path.into()),
            );

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["my_asset.ron"]
    }
}

/// Our secondary asset (can still be used independently).

#[derive(Debug, Clone, Deserialize, TypeUuid, TypePath)]
#[uuid = "618ce0b5-3cb1-4635-bfb4-3c1f57e8463b"]
struct MyOtherAsset {
    value: bool,
}

/// Our secondary asset loader.

#[derive(Default)]
struct MyOtherAssetLoader;

impl AssetLoader for MyOtherAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let other_asset: MyOtherAsset = ron::de::from_bytes(bytes)?;

            load_context.set_default_asset(LoadedAsset::new(other_asset));

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["my_other_asset.ron"]
    }
}
