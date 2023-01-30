//! This example shows various ways to configure texture materials in 3D.

use std::f32::consts::PI;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

/// sets up a scene with textured entities
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // load a texture and retrieve its aspect ratio
    let texture_handle = asset_server.load("branding/bevy_logo_dark_big.png");
    let aspect = 0.25;

    let quad_width = 8.0;

    // this material renders the texture normally
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // this material modulates the texture to make it red (and slightly transparent)
    let red_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgba(1.0, 0.0, 0.0, 0.5),
        base_color_texture: Some(texture_handle.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // and lets make this one blue! (and also slightly transparent)
    let blue_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgba(0.0, 0.0, 1.0, 0.5),
        base_color_texture: Some(texture_handle),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // textured quad - normal
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
            quad_width,
            quad_width * aspect,
        )))),
        material: material_handle,
        transform: Transform::from_xyz(0.0, 0.0, 1.5)
            .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
        ..default()
    });

    // textured quad - modulated (with texture tiling)
    let mut red_tiled_texture_mesh =
        Mesh::from(shape::Quad::new(Vec2::new(quad_width, quad_width * aspect)));
    update_mesh_uvs_with_tiling(&mut red_tiled_texture_mesh, (1.0, 3.0));
    commands.spawn(PbrBundle {
        mesh: meshes.add(red_tiled_texture_mesh),
        material: red_material_handle,
        transform: Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
        ..default()
    });

    // textured quad - modulated (with texture tiling)
    let mut blue_tiled_texture_mesh =
        Mesh::from(shape::Quad::new(Vec2::new(quad_width, quad_width * aspect)));
    update_mesh_uvs_with_tiling(&mut blue_tiled_texture_mesh, (3.0, 1.0));
    commands.spawn(PbrBundle {
        mesh: meshes.add(blue_tiled_texture_mesh),
        material: blue_material_handle,
        transform: Transform::from_xyz(0.0, 0.0, -1.5)
            .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(3.0, 5.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
