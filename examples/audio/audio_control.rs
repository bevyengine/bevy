//! This example illustrates how to load and play an audio file, and control how it's played.

use bevy::{math::ops, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (update_progress_text, update_speed, pause, mute, volume),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        AudioPlayer::new(asset_server.load("sounds/Windless Slopes.ogg")),
        MyMusic,
    ));

    commands.spawn((
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        ProgressText,
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

#[derive(Component)]
struct ProgressText;

fn update_progress_text(
    music_controller: Single<&AudioSink, With<MyMusic>>,
    mut progress_text: Single<&mut Text, With<ProgressText>>,
) {
    progress_text.0 = format!("Progress: {}s", music_controller.position().as_secs_f32());
}

fn update_speed(music_controller: Query<&AudioSink, With<MyMusic>>, time: Res<Time>) {
    let Ok(sink) = music_controller.single() else {
        return;
    };
    if sink.is_paused() {
        return;
    }

    sink.set_speed((ops::sin(time.elapsed_secs() / 5.0) + 1.0).max(0.1));
}

fn pause(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    music_controller: Query<&AudioSink, With<MyMusic>>,
) {
    let Ok(sink) = music_controller.single() else {
        return;
    };

    if keyboard_input.just_pressed(KeyCode::Space) {
        sink.toggle_playback();
    }
}

fn mute(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut music_controller: Query<&mut AudioSink, With<MyMusic>>,
) {
    let Ok(mut sink) = music_controller.single_mut() else {
        return;
    };

    if keyboard_input.just_pressed(KeyCode::KeyM) {
        sink.toggle_mute();
    }
}

fn volume(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut music_controller: Query<&mut AudioSink, With<MyMusic>>,
) {
    let Ok(mut sink) = music_controller.single_mut() else {
        return;
    };

    if keyboard_input.just_pressed(KeyCode::Equal) {
        let current_volume = sink.volume();
        sink.set_volume(current_volume.increase_by_percentage(10.0));
    } else if keyboard_input.just_pressed(KeyCode::Minus) {
        let current_volume = sink.volume();
        sink.set_volume(current_volume.increase_by_percentage(-10.0));
    }
}
