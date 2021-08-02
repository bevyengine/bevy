use bevy::asset::LoadState;
use bevy::prelude::*;

/// This example illustrates how to load and play an audio file on repeat.
#[derive(Default)]
struct OverworldTheme {
    track: Handle<AudioSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Setup,
    Finished,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<OverworldTheme>()
        .add_state(AppState::Setup)
        .add_system_set(SystemSet::on_enter(AppState::Setup).with_system(load_theme))
        .add_system_set(SystemSet::on_update(AppState::Setup).with_system(check_loadstate))
        .add_system_set(SystemSet::on_enter(AppState::Finished).with_system(start_theme))
        .run();
}

fn load_theme(mut overworld_theme: ResMut<OverworldTheme>, asset_server: Res<AssetServer>) {
    overworld_theme.track = asset_server.load("sounds/loop_me_melancholy.mp3");
}

fn check_loadstate(
    mut state: ResMut<State<AppState>>,
    overworld_theme: ResMut<OverworldTheme>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded = asset_server.get_load_state(overworld_theme.track.id) {
        state.set(AppState::Finished).unwrap();
    }
}

fn start_theme(
    audio: Res<Audio>,
    overworld_theme: Res<OverworldTheme>,
    mut assets: ResMut<Assets<AudioSource>>,
) {
    let audio_source = assets.get_mut(&overworld_theme.track).unwrap();
    audio_source.set_repeat_infinite(true);
    audio.play(overworld_theme.track.clone())
}
