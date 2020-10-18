use bevy::prelude::*;

/// This example illustrates how to load and play an audio file
fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(asset_server: Res<AssetServer>, audio_output: Res<AudioOutput>) {
    let music = asset_server.load("sounds/Windless Slopes.mp3");
    audio_output.play(music);
}
