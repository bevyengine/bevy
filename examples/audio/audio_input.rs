//! This example demonstrates Bevy's audio input system for accessing microphone input.

use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (oscilloscope, push_to_listen))
        .run();
}

fn push_to_listen(mut commands: Commands, keyboard_input: Res<ButtonInput<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        // This will use the default audio input device.
        // To choose a different device, see AudioInputOptions.
        commands.start_recording_audio();
    } else if keyboard_input.just_released(KeyCode::Space) {
        commands.stop_recording_audio();
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(
        TextBundle::from_section("Hold the Spacebar to Record!", default()).with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        }),
    );
}

fn oscilloscope(
    mut gizmos: Gizmos,
    mut inputs: EventReader<AudioInputEvent>,
    windows: Query<&Window>,
) {
    let window = windows.single();

    let width = window.width();
    let height = window.height();

    for input in inputs.read() {
        let length = input.iter().count() as f32;
        let channels = input.config.channels as usize;

        let channel_height = height / (2.0 + channels as f32);

        let scale_x = width / length;
        let scale_y = channel_height / 2.0;
        let base_x = -width / 2.0;
        let base_y = scale_y;

        for channel in 0..channels {
            let nodes = input
                .iter_channel(channel)
                .enumerate()
                .map(|(index, sample)| {
                    Vec2::new(
                        base_x + index as f32 * scale_x,
                        base_y + sample * scale_y - channel_height * channel as f32,
                    )
                });

            gizmos.linestrip_2d(nodes, Srgba::GREEN);
        }
    }
}
