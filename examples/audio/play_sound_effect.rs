//! This example illustrates how to play a sound effect on an event.
//! In this case, we will play a sound effect when the space key is pressed.
use bevy::prelude::*;

#[derive(Resource, Deref)]
struct SoundEffect {
    handle: Handle<AudioSource>,
}

// We can setup the logic for how to load our assets in the `FromWorld` trait.
// This code is called via `init_resource`.
impl FromWorld for SoundEffect {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        SoundEffect {
            handle: asset_server.load("sounds/breakout_collision.ogg"),
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SoundEffect>()
        .add_systems(Startup, setup)
        .add_systems(Update, keyboard_event)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    // example instruction
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
