//! Shows how to zoom and orbit an orthographic projection camera.

use std::ops::Range;

use bevy::{input::mouse::MouseWheel, prelude::*, render::camera::ScalingMode};

// Multiply mouse movements by these factors
const CAMERA_ORBIT_SPEED: f32 = 0.02;
const CAMERA_ZOOM_SPEED: f32 = 1.0;

// Clamp fixed vertical scale to this range
const CAMERA_ZOOM_RANGE: Range<f32> = 5.0..50.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, camera_controls)
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Find the middle of the zoom range
    let initial_scale = (CAMERA_ZOOM_RANGE.end - CAMERA_ZOOM_RANGE.start) / 2.0;

    commands.spawn(Camera3dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical(initial_scale),
            ..default()
        }
        .into(),
        transform: Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(5.0, 5.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });

    // Cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::default()),
        material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
        transform: Transform::from_xyz(1.5, 0.5, 1.5),
        ..default()
    });

    // Light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(3.0, 8.0, 5.0),
        ..default()
    });

    info!("Zoom in and out with mouse wheel.");
    info!("Orbit camera with A and D.");
}

fn camera_controls(
    mut camera: Query<(&mut Projection, &mut Transform), With<Camera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel_input: EventReader<MouseWheel>,
) {
    let mut delta_orbit = 0.0;
    if keyboard_input.pressed(KeyCode::KeyA) {
        // Orbit left
        delta_orbit -= CAMERA_ORBIT_SPEED;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        // Orbit right
        delta_orbit += CAMERA_ORBIT_SPEED;
    }

    let (mut projection, mut transform) = camera.single_mut();

    if delta_orbit != 0.0 {
        // Orbit the camera around a fixed point, facing its center.
        transform.translate_around(Vec3::ZERO, Quat::from_axis_angle(Vec3::Y, delta_orbit));
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }

    // Accumulate mouse wheel inputs for this tick
    // TODO: going away in 0.15 with AccumulatedMouseScroll
    let delta_zoom: f32 = mouse_wheel_input.read().map(|e| e.y).sum();
    if delta_zoom == 0.0 {
        return;
    }

    if let Projection::Orthographic(orthographic) = &mut *projection {
        // Get the current scaling_mode value to allow clamping the new value to our zoom range.
        let ScalingMode::FixedVertical(current) = orthographic.scaling_mode else {
            return;
        };
        // Set a new ScalingMode, clamped to a limited range.
        let zoom_level = (current + CAMERA_ZOOM_SPEED * delta_zoom)
            .clamp(CAMERA_ZOOM_RANGE.start, CAMERA_ZOOM_RANGE.end);
        orthographic.scaling_mode = ScalingMode::FixedVertical(zoom_level);
    }
}
