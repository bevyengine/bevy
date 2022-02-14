use bevy::audio::AudioSink;
use bevy::prelude::*;

/// This example illustrates how to load and play an audio file, and control how it's played
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update_speed)
        .add_system(pause)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
    audio_sinks: Res<Assets<AudioSink>>,
) {
    let music = asset_server.load("sounds/Windless Slopes.ogg");
    let handle = audio_sinks.get_handle(audio.play(music));
    commands.insert_resource(MusicControler(handle));
}

struct MusicControler(Handle<AudioSink>);

fn update_speed(
    audio_sinks: Res<Assets<AudioSink>>,
    music_controler: Res<MusicControler>,
    time: Res<Time>,
) {
    if let Some(sink) = audio_sinks.get(&music_controler.0) {
        sink.set_speed(((time.seconds_since_startup() / 5.0).sin() as f32 + 1.0).max(0.1));
    }
}

fn pause(
    keyboard_input: Res<Input<KeyCode>>,
    audio_sinks: Res<Assets<AudioSink>>,
    music_controler: Res<MusicControler>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        if let Some(sink) = audio_sinks.get(&music_controler.0) {
            if sink.is_paused() {
                sink.play()
            } else {
                sink.pause()
            }
        }
    }
}
