//! Shows different built-in plugins that logs diagnostics, like frames per second (FPS), to the console.

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((
            // The diagnostics plugins need to be added after DefaultPlugins as they use e.g. the time plugin for timestamps.
            DefaultPlugins,
            // Adds a system that prints diagnostics to the console.
            // The other diagnostics plugins can still be used without this if you want to use them in an ingame overlay for example.
            LogDiagnosticsPlugin::default(),
            // Adds frame time, FPS and frame count diagnostics.
            FrameTimeDiagnosticsPlugin::default(),
            // Adds an entity count diagnostic.
            bevy::diagnostic::EntityCountDiagnosticsPlugin,
            // Adds cpu and memory usage diagnostics for systems and the entire game process.
            bevy::diagnostic::SystemInformationDiagnosticsPlugin,
            // Forwards various diagnostics from the render app to the main app.
            // These are pretty verbose but can be useful to pinpoint performance issues.
            bevy_render::diagnostic::RenderDiagnosticsPlugin,
        ))
        // No rendering diagnostics are emitted unless something is drawn to the screen,
        // so we spawn a small scene.
        .add_systems(Startup, setup)
        .run();
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
    ));
}
