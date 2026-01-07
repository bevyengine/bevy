//! Example showing how to use the debug buffer visualization system.

use bevy::prelude::*;
use bevy::render::experimental::occlusion_culling::OcclusionCulling;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // Cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Enable prepasses for debugging
        bevy::core_pipeline::prepass::DepthPrepass,
        bevy::core_pipeline::prepass::NormalPrepass,
        bevy::core_pipeline::prepass::MotionVectorPrepass,
        // Enable occlusion culling for Depth Pyramid
        OcclusionCulling,
    ));

    println!("Controls:");
    println!("F1: Cycle Debug Modes and Mips");
    println!("F2: Cycle Opacity (0.8, 0.95, 1.0)");
    println!("F12: Toggle Overlay");
    println!("+/-: Adjust Opacity");
}
