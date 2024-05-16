//! Shows how to create a 2D top-down camera to smoothly follow the player.

use bevy::core_pipeline::bloom::BloomSettings;
use bevy::log::{Level, LogPlugin};
use bevy::math::{vec2, vec3};
use bevy::prelude::*;
use bevy::sprite::{Material2d, MaterialMesh2dBundle, Mesh2dHandle};

/// Player movement speed factor.
const PLAYER_SPEED: f32 = 100.;

/// Camera lerp factor.
const CAM_LERP_FACTOR: f32 = 2.;

#[derive(Component)]
struct Player;

#[derive(Bundle)]
struct PlayerBundle<M: Material2d> {
    player: Player,
    mesh_bundle: MaterialMesh2dBundle<M>,
}

fn main() {
    let default_plugins = DefaultPlugins.set(LogPlugin {
        filter: "info,wgpu_core=warn,wgpu_hal=warn,2d_top_down=debug".into(),
        level: Level::INFO,
        ..default()
    });

    App::new()
        .add_plugins(default_plugins)
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, (scene_setup, camera_setup).chain())
        .add_systems(Update, (move_player, update_camera).chain())
        .run();
}

fn scene_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // World where we move the player
    commands.spawn(MaterialMesh2dBundle {
        mesh: Mesh2dHandle(meshes.add(Rectangle::new(1000., 700.))),
        material: materials.add(Color::srgb(52. / 255., 39. / 255., 69. / 255.)), //77 57 102
        ..default()
    });

    // Player
    commands.spawn(PlayerBundle {
        player: Player,
        mesh_bundle: MaterialMesh2dBundle {
            mesh: meshes.add(Circle::new(25.)).into(),
            material: materials.add(Color::srgb(6.25, 9.4, 9.1)),
            transform: Transform {
                translation: vec3(0., 0., 2.),
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
        camera: Camera {
            hdr: true,
            ..default()
        },
        ..default()
    };

    commands.spawn((camera, BloomSettings::NATURAL));

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

    // Move camera with a smooth effect
    camera.translation = camera
        .translation
        .lerp(direction, time.delta_seconds() * CAM_LERP_FACTOR);
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

    let move_delta = direction * PLAYER_SPEED * time.delta_seconds();
    player.translation += move_delta.extend(0.);
}
