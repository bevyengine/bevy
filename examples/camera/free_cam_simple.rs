//! A simple 3D scene showcasing how to use the `FreeCam` camera controller.
//! This example is primarily for debugging the changes made for issue #21456.

use bevy::camera_controller::free_cam::{FreeCam, FreeCamPlugin, FreeCamState};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FreeCamPlugin) // Adds the PanCam plugin to enable camera panning and zooming controls.
        .add_systems(Startup, (setup, spawn_text).chain())
        .run();
}

fn spawn_text(mut commands: Commands, camera: Query<&FreeCam>) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(-16),
            left: px(12),
            ..default()
        },
        children![Text::new(format!("{}", camera.single().unwrap()))],
    ));
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Spawn a 3D Camera with default FreeCam settings and initialize state
        FreeCam::default(),
        FreeCamState::default(),
    ));
}
