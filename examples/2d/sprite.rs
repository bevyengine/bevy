//! Displays a single [`Sprite`], created from an image.

/// This example uses a png from the assets subdirectory
const BEVY_BIRD_ASSET_PATH: &str = "branding/bevy_bird_dark.png";

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: asset_server.load(BEVY_BIRD_ASSET_PATH),
        ..default()
    });
}
