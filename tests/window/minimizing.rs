//! A test to confirm that `bevy` allows minimizing the window
//! This is run in CI to ensure that this doesn't regress again.
use bevy::{diagnostic::FrameCount, prelude::*};

fn main() {
    // TODO: Combine this with `resizing` once multiple_windows is simpler than
    // it is currently.
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minimizing".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_3d, setup_2d))
        .add_systems(Update, minimize_automatically)
        .run();
}

fn minimize_automatically(mut window: Single<&mut Window>, frames: Res<FrameCount>) {
    if frames.0 != 60 {
        return;
    }

    window.set_minimized(true);
}

/// A simple 3d scene, taken from the `3d_scene` example
fn setup_3d(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
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
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// A simple 2d scene, taken from the `rect` example
fn setup_2d(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            // render the 2d camera after the 3d camera
            order: 1,
            // do not use a clear color
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
    commands.spawn(Sprite::from_color(
        Color::srgb(0.25, 0.25, 0.75),
        Vec2::new(50.0, 50.0),
    ));
}
