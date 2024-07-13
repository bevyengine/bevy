//! Example of a custom asset bundle

use bevy::asset::io::pack::{AssetPack, AssetPackPlugin, GetPack};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AssetPackPlugin::<ExampleAssetPack>::default())
        .add_systems(Startup, setup)
        .run();
}

#[derive(AssetPack)]
struct ExampleAssetPack {
    #[embedded("files/bevy_pixel_dark.png")]
    sprite: Handle<Image>,
}

fn setup(mut commands: Commands, assets: GetPack<ExampleAssetPack>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(SpriteBundle {
        texture: assets.get().sprite.clone(),
        ..default()
    });
}
