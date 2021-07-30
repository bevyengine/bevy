use bevy::prelude::*;

/// This example illustrates how to load and play an audio file on repeat.

struct OverworldThemeTimer(Timer);

struct OverworldTheme {
    track: Handle<AudioSource>,
}

const THEME_LENGTH: f32 = 13.;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(OverworldThemeTimer(Timer::from_seconds(THEME_LENGTH, true)))
        .init_resource::<OverworldTheme>()
        .add_startup_system(play_theme.system())
        .add_system(repeat_theme.system())
        .run();
}

impl FromWorld for OverworldTheme {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource_mut::<AssetServer>().unwrap();
        OverworldTheme {
            track: asset_server.load("sounds/loop_me_melancholy.mp3"),
        }
    }
}

fn play_theme(audio: Res<Audio>, theme: Res<OverworldTheme>) {
    audio.play(theme.track.clone());
}

fn repeat_theme(
    time: Res<Time>,
    mut timer: ResMut<OverworldThemeTimer>,
    audio: Res<Audio>,
    theme: Res<OverworldTheme>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        play_theme(audio, theme);
    }
}
