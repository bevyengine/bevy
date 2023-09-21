//! Loads animations from a skinned glTF, spawns many of them, and plays the
//! animation to stress test skinned meshes.

use std::f32::consts::PI;
use std::time::Duration;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

#[derive(Resource)]
struct Foxes {
    count: usize,
    speed: f32,
    moving: bool,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "🦊🦊🦊 Many Foxes! 🦊🦊🦊".into(),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(Foxes {
            count: std::env::args()
                .nth(1)
                .map_or(1000, |s| s.parse::<usize>().unwrap()),
            speed: 2.0,
            moving: true,
        })
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                setup_scene_once_loaded,
                keyboard_animation_control,
                update_fox_rings.after(keyboard_animation_control),
            ),
        )
        .run();
}

#[derive(Resource)]
struct Animations(Vec<Handle<AnimationClip>>);

const RING_SPACING: f32 = 2.0;
const FOX_SPACING: f32 = 2.0;

#[derive(Component, Clone, Copy)]
enum RotationDirection {
    CounterClockwise,
    Clockwise,
}

impl RotationDirection {
    fn sign(&self) -> f32 {
        match self {
            RotationDirection::CounterClockwise => 1.0,
            RotationDirection::Clockwise => -1.0,
        }
    }
}

#[derive(Component)]
struct Ring {
    radius: f32,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    foxes: Res<Foxes>,
) {
    warn!(include_str!("warning_string.txt"));

    // Insert a resource with the current scene information
    commands.insert_resource(Animations(vec![
        asset_server.load("models/animated/Fox.glb#Animation2"),
        asset_server.load("models/animated/Fox.glb#Animation1"),
        asset_server.load("models/animated/Fox.glb#Animation0"),
    ]));

    // Foxes
    // Concentric rings of foxes, running in opposite directions. The rings are spaced at 2m radius intervals.
    // The foxes in each ring are spaced at least 2m apart around its circumference.'

    // NOTE: This fox model faces +z
    let fox_handle = asset_server.load("models/animated/Fox.glb#Scene0");

    let ring_directions = [
        (
            Quat::from_rotation_y(PI),
            RotationDirection::CounterClockwise,
        ),
        (Quat::IDENTITY, RotationDirection::Clockwise),
    ];

    let mut ring_index = 0;
    let mut radius = RING_SPACING;
    let mut foxes_remaining = foxes.count;

    info!("Spawning {} foxes...", foxes.count);

    while foxes_remaining > 0 {
        let (base_rotation, ring_direction) = ring_directions[ring_index % 2];
        let ring_parent = commands
            .spawn((
                SpatialBundle::INHERITED_IDENTITY,
                ring_direction,
                Ring { radius },
            ))
            .id();

        let circumference = PI * 2. * radius;
        let foxes_in_ring = ((circumference / FOX_SPACING) as usize).min(foxes_remaining);
        let fox_spacing_angle = circumference / (foxes_in_ring as f32 * radius);

        for fox_i in 0..foxes_in_ring {
            let fox_angle = fox_i as f32 * fox_spacing_angle;
            let (s, c) = fox_angle.sin_cos();
            let (x, z) = (radius * c, radius * s);

            commands.entity(ring_parent).with_children(|builder| {
                builder.spawn(SceneBundle {
                    scene: fox_handle.clone(),
                    transform: Transform::from_xyz(x, 0.0, z)
                        .with_scale(Vec3::splat(0.01))
                        .with_rotation(base_rotation * Quat::from_rotation_y(-fox_angle)),
                    ..default()
                });
            });
        }

        foxes_remaining -= foxes_in_ring;
        radius += RING_SPACING;
        ring_index += 1;
    }

    // Camera
    let zoom = 0.8;
    let translation = Vec3::new(
        radius * 1.25 * zoom,
        radius * 0.5 * zoom,
        radius * 1.5 * zoom,
    );
    commands.spawn(Camera3dBundle {
        transform: Transform::from_translation(translation)
            .looking_at(0.2 * Vec3::new(translation.x, 0.0, translation.z), Vec3::Y),
        ..default()
    });

    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5000.0).into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // Light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            first_cascade_far_bound: 0.9 * radius,
            maximum_distance: 2.8 * radius,
            ..default()
        }
        .into(),
        ..default()
    });

    println!("Animation controls:");
    println!("  - spacebar: play / pause");
    println!("  - arrow up / down: speed up / slow down animation playback");
    println!("  - arrow left / right: seek backward / forward");
    println!("  - return: change animation");
}

// Once the scene is loaded, start the animation
fn setup_scene_once_loaded(
    animations: Res<Animations>,
    foxes: Res<Foxes>,
    mut player: Query<&mut AnimationPlayer>,
    mut done: Local<bool>,
) {
    if !*done && player.iter().len() == foxes.count {
        for mut player in &mut player {
            player.play(animations.0[0].clone_weak()).repeat();
        }
        *done = true;
    }
}

fn update_fox_rings(
    time: Res<Time>,
    foxes: Res<Foxes>,
    mut rings: Query<(&Ring, &RotationDirection, &mut Transform)>,
) {
    if !foxes.moving {
        return;
    }

    let dt = time.delta_seconds();
    for (ring, rotation_direction, mut transform) in &mut rings {
        let angular_velocity = foxes.speed / ring.radius;
        transform.rotate_y(rotation_direction.sign() * angular_velocity * dt);
    }
}

fn keyboard_animation_control(
    keyboard_input: Res<Input<KeyCode>>,
    mut animation_player: Query<&mut AnimationPlayer>,
    animations: Res<Animations>,
    mut current_animation: Local<usize>,
    mut foxes: ResMut<Foxes>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        foxes.moving = !foxes.moving;
    }

    if keyboard_input.just_pressed(KeyCode::Up) {
        foxes.speed *= 1.25;
    }

    if keyboard_input.just_pressed(KeyCode::Down) {
        foxes.speed *= 0.8;
    }

    if keyboard_input.just_pressed(KeyCode::Return) {
        *current_animation = (*current_animation + 1) % animations.0.len();
    }

    for mut player in &mut animation_player {
        if keyboard_input.just_pressed(KeyCode::Space) {
            if player.is_paused() {
                player.resume();
            } else {
                player.pause();
            }
        }

        if keyboard_input.just_pressed(KeyCode::Up) {
            let speed = player.speed();
            player.set_speed(speed * 1.25);
        }

        if keyboard_input.just_pressed(KeyCode::Down) {
            let speed = player.speed();
            player.set_speed(speed * 0.8);
        }

        if keyboard_input.just_pressed(KeyCode::Left) {
            let elapsed = player.seek_time();
            player.seek_to(elapsed - 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::Right) {
            let elapsed = player.seek_time();
            player.seek_to(elapsed + 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::Return) {
            player
                .play_with_transition(
                    animations.0[*current_animation].clone_weak(),
                    Duration::from_millis(250),
                )
                .repeat();
        }
    }
}
