//! This example illustrates how to load and play an audio file.
//! For loading additional audio formats, you can enable the corresponding feature for that audio format.

use bevy::prelude::*;

/// This example uses an audio file from the assets subdirectory
const MUSIC_PATH: &str = "sounds/Windless Slopes.ogg";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(AudioBundle {
        source: asset_server.load(MUSIC_PATH),
        ..default()
    });
}
