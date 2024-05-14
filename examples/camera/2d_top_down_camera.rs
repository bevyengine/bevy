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

static MOVE_SPEED: f32 = 100.;
static LERP_FACTOR: f32 = 3.;

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
        ..default()
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

    camera.translation = camera
        .translation
        .lerp(direction, time.delta_seconds() * LERP_FACTOR);
}

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

    let move_delta = direction * MOVE_SPEED * time.delta_seconds();
    player.translation += move_delta.extend(0.);
}
