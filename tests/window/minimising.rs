//! A test to confirm that `bevy` allows minimising the window
//! This is run in CI to ensure that this doesn't regress again.
use bevy::prelude::*;

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
        mesh: meshes.add(Plane3d::default().mesh().size(5.0, 5.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::default()),
        material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
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
            // do not use a clear color
            clear_color: ClearColorConfig::None,
            ..default()
        },
        ..default()
    });
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::srgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(50.0, 50.0)),
            ..default()
        },
        ..default()
    });
}
