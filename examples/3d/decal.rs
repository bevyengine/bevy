//! Decal rendering.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{
    core_pipeline::prepass::DepthPrepass,
    pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt},
    prelude::*,
};
use camera_controller::{CameraController, CameraControllerPlugin};
use std::f32::consts::PI;

// TODO: Showcase a custom material

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
    commands.spawn((
        Name::new("Decal"),
        ForwardDecal,
        MeshMaterial3d(decal_standard_materials.add(ForwardDecalMaterial {
            base: StandardMaterial {
                base_color_texture: Some(asset_server.load("textures/uv_checker_bw.png")),
                ..default()
            },
            extension: ForwardDecalMaterialExt {
                depth_fade_factor: 8.0,
            },
        })),
        Transform::from_translation(Vec3::new(0.15, 0.45, 0.0))
            .with_scale(Vec3::splat(2.0))
            .with_rotation(Quat::from_rotation_z(PI / 4.0)),
    ));

    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        CameraController::default(),
        DepthPrepass, // Must enable the depth prepass to render forward decals
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2000.0,
            ..default()
        },
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let white_material = standard_materials.add(Color::WHITE);

    commands.spawn((
        Name::new("Wall"),
        Mesh3d(meshes.add(Cuboid::new(1.0, 4.0, 3.0))),
        MeshMaterial3d(white_material.clone()),
        Transform::from_xyz(1.0, 0.0, 0.0),
    ));

    commands.spawn((
        Name::new("Floor"),
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(white_material),
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
}
