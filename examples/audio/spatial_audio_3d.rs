//! This example illustrates how to load and play an audio file, and control where the sounds seems to come from.
use bevy::{
    color::palettes::basic::{BLUE, LIME, RED},
    prelude::*,
    time::Stopwatch,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_positions)
        .add_systems(Update, update_listener)
        .add_systems(Update, mute)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Space between the two ears
    let gap = 4.0;

    // sound emitter
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.2).mesh().uv(32, 18))),
        MeshMaterial3d(materials.add(Color::from(BLUE))),
        Transform::from_xyz(0.0, 0.0, 0.0),
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
            // left ear indicator
            (
                Mesh3d(meshes.add(Cuboid::new(0.2, 0.2, 0.2))),
                MeshMaterial3d(materials.add(Color::from(RED))),
                Transform::from_translation(listener.left_ear_offset),
            ),
            // right ear indicator
            (
                Mesh3d(meshes.add(Cuboid::new(0.2, 0.2, 0.2))),
                MeshMaterial3d(materials.add(Color::from(LIME))),
                Transform::from_translation(listener.right_ear_offset),
            )
        ],
    ));

    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // example instructions
    commands.spawn((
        Text::new(
            "Up/Down/Left/Right: Move Listener\nSpace: Toggle Emitter Movement\nM: Toggle Mute",
        ),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Component, Default)]
struct Emitter {
    stopwatch: Stopwatch,
}

fn update_positions(
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
            emitter_transform.translation.x = ops::sin(emitter.stopwatch.elapsed_secs()) * 3.0;
            emitter_transform.translation.z = ops::cos(emitter.stopwatch.elapsed_secs()) * 3.0;
        }
    }
}

fn update_listener(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut listeners: Single<&mut Transform, With<SpatialListener>>,
) {
    let speed = 2.;

    if keyboard.pressed(KeyCode::ArrowRight) {
        listeners.translation.x += speed * time.delta_secs();
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        listeners.translation.x -= speed * time.delta_secs();
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        listeners.translation.z += speed * time.delta_secs();
    }
    if keyboard.pressed(KeyCode::ArrowUp) {
        listeners.translation.z -= speed * time.delta_secs();
    }
}

fn mute(keyboard_input: Res<ButtonInput<KeyCode>>, mut sinks: Query<&mut SpatialAudioSink>) {
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        for mut sink in sinks.iter_mut() {
            sink.toggle_mute();
        }
    }
}
