//! Decal rendering.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{
    core_pipeline::prepass::DepthPrepass,
    pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt},
    prelude::*,
};
use camera_controller::{CameraController, CameraControllerPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CameraControllerPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut decal_standard_materials: ResMut<Assets<ForwardDecalMaterial<StandardMaterial>>>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn((
            Name::new("Sphere"),
            Mesh3d(meshes.add(Sphere::new(1.0))),
            MeshMaterial3d(standard_materials.add(Color::srgb_u8(124, 144, 255))),
            Transform::from_xyz(0.0, 0.5, 0.0),
        ))
        .with_child((
            Name::new("Decal"),
            ForwardDecal,
            MeshMaterial3d(decal_standard_materials.add(ForwardDecalMaterial {
                base: StandardMaterial {
                    base_color_texture: Some(asset_server.load("branding/bevy_logo_dark.png")),
                    ..default()
                },
                extension: ForwardDecalMaterialExt {
                    depth_fade_factor: 8.0,
                },
            })),
            Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)).with_scale(Vec3::splat(0.5)),
        ));

    commands.spawn((
        Name::new("Floor"),
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(standard_materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    commands.spawn((
        Name::new("Light"),
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        CameraController::default(),
        DepthPrepass,
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
