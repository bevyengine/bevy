//! Uses two windows to visualize a 3D model from different angles.

use bevy::{prelude::*, render::camera::RenderTarget, window::PresentMode};

fn main() {
    App::new()
        // Primary window gets spawned as a result of `DefaultPlugins`
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_scene)
        .add_startup_system(setup_second_window)
        .add_system(bevy::window::close_on_esc)
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // add entities to the world
    commands.spawn_scene(asset_server.load("models/monkey/Monkey.gltf#Scene0"));
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 4.0),
        ..default()
    });
    // main camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn setup_second_window(mut commands: Commands) {
    // Spawn a new entity that will act as our window id
    let window_id = commands.spawn().id();

    // Send a command to spawn a new window on this entity
    commands.window(window_id).create_window(WindowDescriptor {
        width: 800.,
        height: 600.,
        present_mode: PresentMode::Immediate,
        title: "Second window".to_string(),
        ..default()
    });

    // second window camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            target: RenderTarget::Window(window_id),
            ..default()
        },
        ..default()
    });
}
