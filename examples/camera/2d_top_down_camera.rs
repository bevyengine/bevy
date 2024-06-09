//! This example showcases a 2D top-down camera with smooth player tracking.
//!
//! ## Controls
//!
//! | Key Binding          | Action        |
//! |:---------------------|:--------------|
//! | `Z`(azerty), `W`(US) | Move forward  |
//! | `S`                  | Move backward |
//! | `Q`(azerty), `A`(US) | Move left     |
//! | `D`                  | Move right    |

use bevy::core_pipeline::bloom::BloomSettings;
use bevy::log::{Level, LogPlugin};
use bevy::math::{vec2, vec3};
use bevy::prelude::*;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};

/// Player movement speed factor.
const PLAYER_SPEED: f32 = 100.;

/// Camera lerp factor.
const CAM_LERP_FACTOR: f32 = 2.;

#[derive(Component)]
struct Player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "info,wgpu_core=warn,wgpu_hal=warn,2d_top_down=debug".into(),
            level: Level::INFO,
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(
            Startup,
            (setup_scene, setup_instructions, setup_camera).chain(),
        )
        .add_systems(Update, (move_player, update_camera).chain())
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // World where we move the player
    commands.spawn(MaterialMesh2dBundle {
        mesh: Mesh2dHandle(meshes.add(Rectangle::new(1000., 700.))),
        material: materials.add(Color::srgb(0.2, 0.2, 0.3)),
        ..default()
    });

    // Player
    commands.spawn((
        Player,
        MaterialMesh2dBundle {
            mesh: meshes.add(Circle::new(25.)).into(),
            material: materials.add(Color::srgb(6.25, 9.4, 9.1)),
            transform: Transform {
                translation: vec3(0., 0., 2.),
                ..default()
            },
            ..default()
        },
    ));

    debug!("Scene setup finished");
}

fn setup_instructions(mut commands: Commands) {
    commands.spawn(
        TextBundle::from_section(
            "Move the light with ZQSD or WASD.\nThe camera will smoothly track the light.",
            TextStyle::default(),
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(0., 0., 1.),
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        BloomSettings::NATURAL,
    ));

    debug!("Camera setup finished");
}

/// Update the camera position by tracking the player.
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

    // Applies a smooth effect to camera movement using interpolation between
    // the camera position and the player position on the x and y axes.
    // Here we use the in-game time (in seconds), to get the elapsed time since
    // the previous time update. This avoids jittery when tracking the player.
    camera.translation = camera
        .translation
        .lerp(direction, time.delta_seconds() * CAM_LERP_FACTOR);
}

/// Update the player position with keyboard inputs.
fn move_player(
    mut player: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
    kb_input: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut player) = player.get_single_mut() else {
        debug!("Player not found");
        return;
    };

    let mut direction = vec2(0., 0.);

    if kb_input.pressed(KeyCode::KeyW) {
        direction.y = 1.;
    }

    if kb_input.pressed(KeyCode::KeyS) {
        direction.y = -1.;
    }

    if kb_input.pressed(KeyCode::KeyA) {
        direction.x = -1.;
    }

    if kb_input.pressed(KeyCode::KeyD) {
        direction.x = 1.;
    }

    // Progressively update the player's position over time. Normalize the
    // direction vector to prevent it from exceeding a magnitude of 1 when
    // moving diagonally.
    let move_delta = direction.normalize_or_zero() * PLAYER_SPEED * time.delta_seconds();
    player.translation += move_delta.extend(0.);
}
