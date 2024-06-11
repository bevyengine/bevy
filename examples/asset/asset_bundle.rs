//! Example of a custom asset bundle

use bevy::asset::bundle::AssetPack;
use bevy::prelude::*;
use bevy_internal::asset::bundle::{AssetPackPlugin, GetPack};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, AssetBundleExamplePlugin))
        .add_systems(Startup, setup)
        .run();
}

struct AssetBundleExamplePlugin;

#[derive(AssetPack)]
struct ExampleAssetPack {
    #[embedded("files/bevy_pixel_dark.png")]
    thing: Handle<Image>,
}

impl Plugin for AssetBundleExamplePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AssetPackPlugin::<ExampleAssetPack>::default());
    }
}

fn setup(mut commands: Commands, assets: GetPack<ExampleAssetPack>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(SpriteBundle {
        texture: assets.get().thing.clone(),
        ..default()
    });
}
