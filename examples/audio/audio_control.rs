//! This example illustrates how to load and play an audio file, and control how it's played.

use bevy::{math::ops, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (update_speed, pause, mute, volume))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        AudioPlayer::new(asset_server.load("sounds/Windless Slopes.ogg")),
        MyMusic,
    ));

    // example instructions
    commands.spawn((
        Text::new("-/=: Volume Down/Up\nSpace: Toggle Playback\nM: Toggle Mute"),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    // camera
    commands.spawn(Camera3d::default());
}

#[derive(Component)]
struct MyMusic;

fn update_speed(sink: Single<&AudioSink, With<MyMusic>>, time: Res<Time>) {
    sink.set_speed((ops::sin(time.elapsed_secs() / 5.0) + 1.0).max(0.1));
}

fn pause(keyboard_input: Res<ButtonInput<KeyCode>>, sink: Single<&AudioSink, With<MyMusic>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        sink.toggle_playback();
    }
}

fn mute(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut sink: Single<&mut AudioSink, With<MyMusic>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        sink.toggle_mute();
    }
}

fn volume(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut sink: Single<&mut AudioSink, With<MyMusic>>,
) {
    if keyboard_input.just_pressed(KeyCode::Equal) {
        let current_volume = sink.volume();
        sink.set_volume(current_volume + 0.1);
    } else if keyboard_input.just_pressed(KeyCode::Minus) {
        let current_volume = sink.volume();
        sink.set_volume(current_volume - 0.1);
    }
}
