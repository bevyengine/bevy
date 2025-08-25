//! Test rendering of many cameras and lights

use std::f32::consts::PI;

use bevy::{
    camera::Viewport,
    math::ops::{cos, sin},
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WinitSettings::continuous())
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
) -> Result {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // lights
    for i in 0..NUM_LIGHTS {
        let angle = (i as f32) / (NUM_LIGHTS as f32) * PI * 2.0;
        commands.spawn((
            PointLight {
                color: Color::hsv(angle.to_degrees(), 1.0, 1.0),
                intensity: 2_000_000.0 / NUM_LIGHTS as f32,
                shadows_enabled: true,
                ..default()
            },
            Transform::from_xyz(sin(angle) * 4.0, 2.0, cos(angle) * 4.0),
        ));
    }

    // cameras
    let window = window.single()?;
    let width = window.resolution.width() / CAMERA_COLS as f32 * window.resolution.scale_factor();
    let height = window.resolution.height() / CAMERA_ROWS as f32 * window.resolution.scale_factor();
    let mut i = 0;
    for y in 0..CAMERA_COLS {
        for x in 0..CAMERA_ROWS {
            let angle = i as f32 / (CAMERA_ROWS * CAMERA_COLS) as f32 * PI * 2.0;
            commands.spawn((
                Camera3d::default(),
                Camera {
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
                Transform::from_xyz(sin(angle) * 4.0, 2.5, cos(angle) * 4.0)
                    .looking_at(Vec3::ZERO, Vec3::Y),
            ));
            i += 1;
        }
    }
    Ok(())
}

fn rotate_cameras(time: Res<Time>, mut query: Query<&mut Transform, With<Camera>>) {
    for mut transform in query.iter_mut() {
        transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_secs()));
    }
}
