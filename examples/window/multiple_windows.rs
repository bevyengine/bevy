//! Uses two windows to visualize a 3D model from different angles.

use bevy::{prelude::*, render::camera::RenderTarget, window::WindowRef};

fn main() {
    App::new()
        // By default, a primary window gets spawned by `WindowPlugin`, contained in `DefaultPlugins`
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // add entities to the world
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/torus/torus.gltf#Scene0"),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 4.0),
        ..default()
    });
    // main camera, cameras default to the primary window
    // so we don't need to specify that.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Spawn a second window
    let second_window = commands
        .spawn(Window {
            title: "Second window".to_owned(),
            ..default()
        })
        .id();

    // second window camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            target: RenderTarget::Window(WindowRef::Entity(second_window)),
            ..default()
        },
        ..default()
    });
}
