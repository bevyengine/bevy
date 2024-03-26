//! Shows how to create a 2D top-down camera to smoothly follow the player.

// Init state, camera and plane
// Cube character movement on a plane: Transform
// Update position on time with delta seconds
// Camera Position: follow player
// 1. Query player
// 2. Query Camera mut
// 3. Update cam position based on player movement
// 4. Optimize for smooth movement with smooth damp
// 5. Level Boundaries: show damp effect on boundaries
// 6. Colliders: to better showcase camera movements (optional)

use bevy::log::{Level, LogPlugin};
use bevy::math::{vec2, vec3};
use bevy::prelude::*;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};

static MOVE_SPEED: f32 = 10.;

#[derive(Component, Debug)]
struct Player;

#[derive(Bundle, Debug)]
struct PlayerBundle {
    player: Player,
    sprite_bundle: SpriteBundle,
}

fn main() {
    let default_plugins = DefaultPlugins.set(LogPlugin {
        filter: "info,wgpu_core=warn,wgpu_hal=warn,2d_top_down=debug".into(),
        level: Level::INFO,
        update_subscriber: None,
    });

    App::new()
        .add_plugins(default_plugins)
        .add_systems(Startup, (scene_setup, camera_setup).chain())
        .add_systems(Update, (update_camera, move_player))
        .run();
}

fn scene_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // World where we move the player
    commands.spawn(MaterialMesh2dBundle {
        mesh: Mesh2dHandle(meshes.add(Rectangle::new(1000.0, 1000.0))),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });

    // Player
    commands.spawn(PlayerBundle {
        player: Player,
        sprite_bundle: SpriteBundle {
            transform: Transform {
                scale: vec3(50., 50., 1.),
                translation: vec3(0., 0., 2.),
                ..default()
            },
            sprite: Sprite {
                color: Color::BLACK,
                ..default()
            },
            ..default()
        },
    });

    debug!("Scene setup finished");
}

fn camera_setup(mut commands: Commands) {
    let camera = Camera2dBundle {
        transform: Transform::from_xyz(0., 0., 1.),
        ..default()
    };

    commands.spawn(camera);

    debug!("Camera setup finished");
}

fn update_camera(
    mut camera: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
    player: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    time: Res<Time>,
) {
    let Ok(mut camera) = camera.get_single_mut() else {
        debug!("Camera2d not found");
        return;
    };

    let Ok(player) = player.get_single() else {
        debug!("Player not found");
        return;
    };

    let Vec3 { x, y, .. } = player.translation;
    let direction = Vec3::new(x, y, camera.translation.z);

    let smooth_damp = smooth_damp(
        camera.translation,
        direction,
        Vec3::ZERO,
        0.2,
        f32::INFINITY,
        time.delta_seconds(),
    );

    camera.translation = smooth_damp;
}

fn move_player(
    mut player: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
    _kb_input: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut player) = player.get_single_mut() else {
        debug!("Player not found");
        return;
    };

    let move_delta = vec2(1., 1.) * MOVE_SPEED * time.delta_seconds();
    player.translation += move_delta.extend(0.);
}

/// Update a vector `Vec3` towards a target over time (`delta_time`).
/// The smoothness is achieved with a spring-damper like function.
///
/// Algorithm based on Game Programming Gems vol.4,
/// chapter 1.10 "Critically Damped Ease-In/Ease-Out Smoothing".
pub fn smooth_damp(
    from: Vec3,
    to: Vec3,
    mut velocity: Vec3,
    mut smoothness: f32,
    max_speed: f32,
    delta_time: f32,
) -> Vec3 {
    // The desired smoothness clamped to the minimum value.
    smoothness = f32::max(0.0001, smoothness);
    // Corresponds to the spring's natural frequency
    let omega = 2. / smoothness;

    let x = omega * delta_time;
    // Approximation
    let exp = 1. / (1. + x + 0.48 * x * x + 0.235 * x * x * x);

    let mut distance_x = from.x - to.x;
    let mut distance_y = from.y - to.y;
    let mut distance_z = from.z - to.z;

    let max_distance = max_speed * smoothness;
    distance_x = f32::min(f32::max(-max_distance, distance_x), max_distance);
    distance_y = f32::min(f32::max(-max_distance, distance_y), max_distance);
    distance_z = f32::min(f32::max(-max_distance, distance_z), max_distance);

    let temp_x = (velocity.x + omega * distance_x) * delta_time;
    let temp_y = (velocity.y + omega * distance_y) * delta_time;
    let temp_z = (velocity.z + omega * distance_z) * delta_time;

    velocity.x = (velocity.x - omega * temp_x) * exp;
    velocity.y = (velocity.y - omega * temp_y) * exp;
    velocity.z = (velocity.z - omega * temp_z) * exp;

    let x = to.x + (distance_x + temp_x) * exp;
    let y = to.y + (distance_y + temp_y) * exp;
    let z = to.z + (distance_z + temp_z) * exp;

    Vec3::new(x, y, z)
}
