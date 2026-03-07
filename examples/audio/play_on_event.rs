//! This example illustrates how to play a sound effect on an event.
//! In this case, we will play a sound effect when the space key is pressed.
use bevy::prelude::*;

#[derive(Resource, Deref)]
struct SoundEffect(Handle<AudioSource>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, keyboard_event)
        .run();
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    // Load an asset as a global resource
    let handle = asset_server.load("sounds/breakout_collision.ogg");
    commands.insert_resource(SoundEffect(handle));

    // example instructions
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new("Press Space to play the sound effect."),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}

// spawn an audio player with the sound effect when the space key is pressed
fn keyboard_event(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    sound_effect: Res<SoundEffect>,
    mut commands: Commands,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        commands.spawn((
            AudioPlayer::new(sound_effect.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}
