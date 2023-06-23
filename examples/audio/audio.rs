//! This example illustrates how to load and play an audio file.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(AudioBundle::from_audio(
        asset_server.load("sounds/Windless Slopes.ogg"),
    ));
}
