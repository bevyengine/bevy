use bevy::prelude::*;

/// This example illustrates how to load and play an audio file
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    let music = asset_server.load("sounds/Windless Slopes.mp3");
    audio.play(music);
}
