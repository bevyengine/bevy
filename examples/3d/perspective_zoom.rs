//! Shows how to zoom and orbit a perspective projection camera.

use std::ops::Range;

use bevy::{input::mouse::MouseWheel, prelude::*};

// Multiply mouse movements by these factors
const CAMERA_ORBIT_SPEED: f32 = 0.02;
const CAMERA_ZOOM_SPEED: f32 = 0.5;

// Clamp FOV to this range. Note that we can't adjust FOV to more than PI, which represents
// a 180 degree field.
const CAMERA_ZOOM_RANGE: Range<f32> = 0.5..3.0;

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
    // Camera3dBundle defaults to using a PerspectiveProjection
    commands.spawn(Camera3dBundle {
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
        transform.translate_around(Vec3::ZERO, Quat::from_axis_angle(Vec3::Y, delta_orbit));
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }

    // Accumulate mouse wheel inputs for this tick
    // TODO: going away in 0.15 with AccumulatedMouseScroll
    let delta_zoom: f32 = mouse_wheel_input.read().map(|e| e.y).sum();
    if delta_zoom == 0.0 {
        return;
    }

    if let Projection::Perspective(perspective) = &mut *projection {
        // Adjust the field of view, but keep it within our stated range
        perspective.fov = (perspective.fov + CAMERA_ZOOM_SPEED * delta_zoom)
            .clamp(CAMERA_ZOOM_RANGE.start, CAMERA_ZOOM_RANGE.end);
    }
}
