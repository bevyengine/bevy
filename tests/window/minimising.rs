//! A test to confirm that `bevy` allows minimising the window
//! This is run in CI to ensure that this doesn't regress again.
use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*};

fn main() {
    // TODO: Combine this with `resizing` once multiple_windows is simpler than
    // it is currently.
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minimising".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_3d, setup_2d))
        .add_systems(Update, minimise_automatically)
        .run();
}

fn minimise_automatically(mut windows: Query<&mut Window>, mut frames: Local<u32>) {
    if *frames == 60 {
        let mut window = windows.single_mut();
        window.set_minimized(true);
    } else {
        *frames += 1;
    }
}

/// A simple 3d scene, taken from the `3d_scene` example
fn setup_3d(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane {
            size: 5.0,
            subdivisions: 0,
        })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

/// A simple 2d scene, taken from the `rect` example
fn setup_2d(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        camera: Camera {
            // render the 2d camera after the 3d camera
            order: 1,
            ..default()
        },
        camera_2d: Camera2d {
            // do not use a clear color
            clear_color: ClearColorConfig::None,
        },
        ..default()
    });
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(50.0, 50.0)),
            ..default()
        },
        ..default()
    });
}
