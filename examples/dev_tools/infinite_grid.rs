//! A simple example to show how to spawn an infinite grid.
//!
//! Infinite grids are useful as the ground plane in editor-like applications,
//! as they provide a consistent reference for the orientation of objects
//! and an evenly spaced grid to judge relative scale.

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FreeCameraPlugin,
            // Make sure to add the plugin
            InfiniteGridPlugin,
        ))
        .add_systems(Startup, setup_system)
        .run();
}

fn setup_system(mut commands: Commands, mut asset_commands: AssetCommands) {
    commands.spawn((
        // You need to spawn an entity with this component
        InfiniteGrid,
        // Optional component you can use to configure the grid
        InfiniteGridSettings::default(),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-12.5, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        FreeCamera::default(),
    ));

    commands.spawn((
        DirectionalLight { ..default() },
        Transform::from_translation(Vec3::X * 15. + Vec3::Y * 20.).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // cube
    commands.spawn((
        Mesh3d(asset_commands.spawn_asset(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)))),
        MeshMaterial3d(
            asset_commands.spawn_asset(StandardMaterial::from_color(Color::srgba(
                1.0, 1.0, 1.0, 0.5,
            ))),
        ),
        Transform::from_xyz(0.0, 2.0, 0.0),
    ));

    commands.spawn((
        Mesh3d(asset_commands.spawn_asset(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)))),
        MeshMaterial3d(
            asset_commands.spawn_asset(StandardMaterial::from_color(Color::srgba(
                1.0, 1.0, 1.0, 0.5,
            ))),
        ),
        Transform::from_xyz(0.0, -2.0, 0.0),
    ));
}
