use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(asset_server: Res<AssetServer>, audio_output: Res<AudioOutput>) {
    let music = asset_server
        .load("assets/sounds/Windless Slopes.mp3")
        .unwrap();
    audio_output.queue(music);
}
