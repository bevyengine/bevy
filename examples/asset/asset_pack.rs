//! Example of a custom asset bundle

use bevy::asset::pack::AssetPack;
use bevy::prelude::*;
use bevy_internal::asset::pack::{AssetPackPlugin, GetPack};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, AssetPackExamplePlugin))
        .add_systems(Startup, setup)
        .run();
}

struct AssetPackExamplePlugin;

#[derive(AssetPack)]
struct ExampleAssetPack {
    #[embedded("files/bevy_pixel_dark.png")]
    sprite: Handle<Image>,
}

impl Plugin for AssetPackExamplePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AssetPackPlugin::<ExampleAssetPack>::default());
    }
}

fn setup(mut commands: Commands, assets: GetPack<ExampleAssetPack>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(SpriteBundle {
        texture: assets.get().sprite.clone(),
        ..default()
    });
}
