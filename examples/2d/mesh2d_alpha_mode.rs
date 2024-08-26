//! This example is used to test how transforms interact with alpha modes for [`MaterialMesh2dBundle`] entities.
//! This makes sure the depth buffer is correctly being used for opaque and transparent 2d meshes

use bevy::{
    color::palettes::css::{BLUE, GREEN, WHITE},
    prelude::*,
    sprite::{AlphaMode2d, MaterialMesh2dBundle},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    let texture_handle = asset_server.load("branding/icon.png");
    let mesh_handle = meshes.add(Rectangle::from_size(Vec2::splat(256.0)));

    // opaque
    // Each sprite should be square with the transparent parts being completely black
    // The blue sprite should be on top with the white and green one behind it
    commands.spawn(MaterialMesh2dBundle {
        mesh: mesh_handle.clone().into(),
        material: materials.add(ColorMaterial {
            color: WHITE.into(),
            alpha_mode: AlphaMode2d::Opaque,
            texture: Some(texture_handle.clone()),
        }),
        transform: Transform::from_xyz(-400.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn(MaterialMesh2dBundle {
        mesh: mesh_handle.clone().into(),
        material: materials.add(ColorMaterial {
            color: BLUE.into(),
            alpha_mode: AlphaMode2d::Opaque,
            texture: Some(texture_handle.clone()),
        }),
        transform: Transform::from_xyz(-300.0, 0.0, 1.0),
        ..default()
    });
    commands.spawn(MaterialMesh2dBundle {
        mesh: mesh_handle.clone().into(),
        material: materials.add(ColorMaterial {
            color: GREEN.into(),
            alpha_mode: AlphaMode2d::Opaque,
            texture: Some(texture_handle.clone()),
        }),
        transform: Transform::from_xyz(-200.0, 0.0, -1.0),
        ..default()
    });

    // Test the interaction between opaque/mask and transparent meshes
    // The white sprite should be:
    // - only the icon is opaque but background is transparent
    // - on top of the green sprite
    // - behind the blue sprite
    commands.spawn(MaterialMesh2dBundle {
        mesh: mesh_handle.clone().into(),
        material: materials.add(ColorMaterial {
            color: WHITE.into(),
            alpha_mode: AlphaMode2d::Mask(0.5),
            texture: Some(texture_handle.clone()),
        }),
        transform: Transform::from_xyz(200.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn(MaterialMesh2dBundle {
        mesh: mesh_handle.clone().into(),
        material: materials.add(ColorMaterial {
            color: BLUE.with_alpha(0.7).into(),
            alpha_mode: AlphaMode2d::Blend,
            texture: Some(texture_handle.clone()),
        }),
        transform: Transform::from_xyz(300.0, 0.0, 1.0),
        ..default()
    });
    commands.spawn(MaterialMesh2dBundle {
        mesh: mesh_handle.clone().into(),
        material: materials.add(ColorMaterial {
            color: GREEN.with_alpha(0.7).into(),
            alpha_mode: AlphaMode2d::Blend,
            texture: Some(texture_handle),
        }),
        transform: Transform::from_xyz(400.0, 0.0, -1.0),
        ..default()
    });
}
