use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(check_if_empty)
        .run();
}

#[derive(Resource)]
pub struct Playing(Handle<AudioSink>);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
    audio_sinks: Res<Assets<AudioSink>>,
) {
    let weak = audio.play(asset_server.load("sounds/breakout_collision.ogg"));
    let strong = audio_sinks.get_handle(weak);

    commands.insert_resource(Playing(strong));
}

fn check_if_empty(
    audio_sinks: Res<Assets<AudioSink>>,
    playing: Res<Playing>,
) {
    if let Some(sink) = audio_sinks.get(&playing.0) {
        if sink.empty() {
            info!("Sink is empty!");
        }
    }
}
