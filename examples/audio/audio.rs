use bevy::prelude::*;

/// This example illustrates how to load and play an audio file
struct Music((Handle<AudioSource>, bool));

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(play_audio.thread_local_system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let music: Handle<AudioSource> = asset_server
        .load("assets/sounds/Windless Slopes.mp3")
        .unwrap();
    commands.insert_resource(Music((music, false)));
}

fn play_audio(_world: &mut World, resources: &mut Resources) {
    let mut music = resources.get_mut::<Music>().unwrap();
    if !music.0.1 {
        let audio_output = resources.get_thread_local::<AudioOutput>().unwrap();
        audio_output.play(music.0.0);
        music.0.1 = true;
    }
}
