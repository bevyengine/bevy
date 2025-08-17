//! This example illustrates how to load and play an audio file, and control where the sounds seems to come from.
use bevy::{
    audio::{AudioPlugin, SpatialScale},
    color::palettes::css::*,
    prelude::*,
    time::Stopwatch,
};

/// Spatial audio uses the distance to attenuate the sound volume. In 2D with the default camera,
/// 1 pixel is 1 unit of distance, so we use a scale so that 100 pixels is 1 unit of distance for
/// audio.
const AUDIO_SCALE: f32 = 1. / 100.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AudioPlugin {
            default_spatial_scale: SpatialScale::new_2d(AUDIO_SCALE),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, update_emitters)
        .add_systems(Update, update_listener)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Space between the two ears
    let gap = 400.0;

    // sound emitter
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(15.0))),
        MeshMaterial2d(materials.add(Color::from(BLUE))),
        Transform::from_translation(Vec3::new(0.0, 50.0, 0.0)),
        Emitter::default(),
        AudioPlayer::new(asset_server.load("sounds/Windless Slopes.ogg")),
        PlaybackSettings::LOOP.with_spatial(true),
    ));

    let listener = SpatialListener::new(gap);
    commands.spawn((
        Transform::default(),
        Visibility::default(),
        listener.clone(),
        children![
            // left ear
            (
                Sprite::from_color(RED, Vec2::splat(20.0)),
                Transform::from_xyz(-gap / 2.0, 0.0, 0.0),
            ),
            // right ear
            (
                Sprite::from_color(LIME, Vec2::splat(20.0)),
                Transform::from_xyz(gap / 2.0, 0.0, 0.0),
            )
        ],
    ));

    // example instructions
    commands.spawn((
        Text::new("Up/Down/Left/Right: Move Listener\nSpace: Toggle Emitter Movement"),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    // camera
    commands.spawn(Camera2d);
}

#[derive(Component, Default)]
struct Emitter {
    stopwatch: Stopwatch,
}

fn update_emitters(
    time: Res<Time>,
    mut emitters: Query<(&mut Transform, &mut Emitter), With<Emitter>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for (mut emitter_transform, mut emitter) in emitters.iter_mut() {
        if keyboard.just_pressed(KeyCode::Space) {
            if emitter.stopwatch.is_paused() {
                emitter.stopwatch.unpause();
            } else {
                emitter.stopwatch.pause();
            }
        }

        emitter.stopwatch.tick(time.delta());

        if !emitter.stopwatch.is_paused() {
            emitter_transform.translation.x = ops::sin(emitter.stopwatch.elapsed_secs()) * 500.0;
        }
    }
}

fn update_listener(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut listener: Single<&mut Transform, With<SpatialListener>>,
) {
    let speed = 200.;

    if keyboard.pressed(KeyCode::ArrowRight) {
        listener.translation.x += speed * time.delta_secs();
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        listener.translation.x -= speed * time.delta_secs();
    }
    if keyboard.pressed(KeyCode::ArrowUp) {
        listener.translation.y += speed * time.delta_secs();
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        listener.translation.y -= speed * time.delta_secs();
    }
}
