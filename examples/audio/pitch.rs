//! This example illustrates how to play a single-frequency sound (aka a pitch)

use bevy::prelude::*;
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_event::<PlayPitch>()
        .add_systems(Startup, setup)
        .add_systems(Update, (play_pitch, keyboard_input_system))
        .run();
}

#[derive(Event, Default)]
struct PlayPitch;

#[derive(Resource)]
struct PitchFrequency(f32);

fn setup(mut commands: Commands) {
    commands.insert_resource(PitchFrequency(220.0));
}

fn play_pitch(
    mut pitch_assets: ResMut<Assets<Pitch>>,
    frequency: Res<PitchFrequency>,
    mut events: EventReader<PlayPitch>,
    mut commands: Commands,
) {
    for _ in events.read() {
        info!("playing pitch with frequency: {}", frequency.0);
        commands.spawn(PitchBundle {
            source: pitch_assets.add(Pitch::new(frequency.0, Duration::new(1, 0))),
            settings: PlaybackSettings::DESPAWN,
        });
        info!("number of pitch assets: {}", pitch_assets.len());
    }
}

fn keyboard_input_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut frequency: ResMut<PitchFrequency>,
    mut events: EventWriter<PlayPitch>,
) {
    if keyboard_input.just_pressed(KeyCode::Up) {
        frequency.0 *= 2.0f32.powf(1.0 / 12.0);
    }
    if keyboard_input.just_pressed(KeyCode::Down) {
        frequency.0 /= 2.0f32.powf(1.0 / 12.0);
    }
    if keyboard_input.just_pressed(KeyCode::Space) {
        events.send(PlayPitch);
    }
}
