//! Test rendering of many cameras and lights

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_render::camera::Viewport;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_cameras)
        .run();
}

const CAMERA_ROWS: usize = 4;
const CAMERA_COLS: usize = 4;
const NUM_LIGHTS: usize = 5;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    window: Query<&Window>,
) {
    // circular base
    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(4.0)),
        material: materials.add(Color::WHITE),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });

    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(Color::WHITE),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    // lights
    for i in 0..NUM_LIGHTS {
        let angle = (i as f32) / (NUM_LIGHTS as f32) * PI * 2.0;
        commands.spawn(PointLightBundle {
            point_light: PointLight {
                color: Color::hsv(angle.to_degrees(), 1.0, 1.0),
                intensity: 2_000_000.0 / NUM_LIGHTS as f32,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(angle.sin() * 4.0, 2.0, angle.cos() * 4.0),
            ..default()
        });
    }

    // cameras
    let window = window.single();
    let width = window.resolution.width() / CAMERA_COLS as f32 * window.resolution.scale_factor();
    let height = window.resolution.height() / CAMERA_ROWS as f32 * window.resolution.scale_factor();
    let mut i = 0;
    for y in 0..CAMERA_COLS {
        for x in 0..CAMERA_ROWS {
            let angle = i as f32 / (CAMERA_ROWS * CAMERA_COLS) as f32 * PI * 2.0;
            commands.spawn(Camera3dBundle {
                transform: Transform::from_xyz(angle.sin() * 4.0, 2.5, angle.cos() * 4.0)
                    .looking_at(Vec3::ZERO, Vec3::Y),
                camera: Camera {
                    viewport: Some(Viewport {
                        physical_position: UVec2::new(
                            (x as f32 * width) as u32,
                            (y as f32 * height) as u32,
                        ),
                        physical_size: UVec2::new(width as u32, height as u32),
                        ..default()
                    }),
                    order: i,
                    ..default()
                },
                ..default()
            });
            i += 1;
        }
    }
}

fn rotate_cameras(time: Res<Time>, mut query: Query<&mut Transform, With<Camera>>) {
    for mut transform in query.iter_mut() {
        transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_seconds()));
    }
}
