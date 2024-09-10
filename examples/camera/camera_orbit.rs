//! Shows how to orbit camera around a static scene using pitch, yaw, and roll.

use std::{f32::consts::FRAC_PI_2, ops::Range};

use bevy::prelude::*;

#[derive(Debug, Default, Resource)]
struct CameraSettings {
    pub orbit_distance: f32,
    // Multiply keyboard inputs by this factor
    pub orbit_speed: f32,
    // Clamp pitch to this range
    pub pitch_range: Range<f32>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<CameraSettings>()
        .add_systems(Startup, (setup, instructions))
        .add_systems(Update, orbit)
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut camera_settings: ResMut<CameraSettings>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Limiting pitch stops some unexpected rotation past 90° up or down.
    let pitch_limit = FRAC_PI_2 - 0.01;

    camera_settings.orbit_distance = 10.0;
    camera_settings.orbit_speed = 1.0;
    camera_settings.pitch_range = -pitch_limit..pitch_limit;

    commands.spawn((
        Name::new("Camera"),
        Camera3dBundle {
            transform: Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
    ));

    commands.spawn((
        Name::new("Plane"),
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(5.0, 5.0)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.5, 0.3),
                // Turning off culling keeps the plane visible when viewed from beneath.
                cull_mode: None,
                ..default()
            }),
            ..default()
        },
    ));

    commands.spawn((
        Name::new("Cube"),
        PbrBundle {
            mesh: meshes.add(Cuboid::default()),
            material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
            transform: Transform::from_xyz(1.5, 0.51, 1.5),
            ..default()
        },
    ));

    commands.spawn((
        Name::new("Light"),
        PointLightBundle {
            transform: Transform::from_xyz(3.0, 8.0, 5.0),
            ..default()
        },
    ));
}

fn instructions(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Instructions"),
            NodeBundle {
                style: Style {
                    align_items: AlignItems::Start,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Start,
                    width: Val::Percent(100.),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "W or S: pitch",
                TextStyle::default(),
            ));
            parent.spawn(TextBundle::from_section(
                "A or D: yaw",
                TextStyle::default(),
            ));
            parent.spawn(TextBundle::from_section(
                "Q or E: roll",
                TextStyle::default(),
            ));
        });
}

fn orbit(
    mut camera: Query<&mut Transform, With<Camera>>,
    camera_settings: Res<CameraSettings>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let mut transform = camera.single_mut();

    let mut delta_pitch = 0.0;
    let mut delta_roll = 0.0;
    let mut delta_yaw = 0.0;

    if keyboard_input.pressed(KeyCode::KeyW) {
        delta_pitch += camera_settings.orbit_speed;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        delta_pitch -= camera_settings.orbit_speed;
    }

    if keyboard_input.pressed(KeyCode::KeyQ) {
        delta_roll -= camera_settings.orbit_speed;
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        delta_roll += camera_settings.orbit_speed;
    }

    if keyboard_input.pressed(KeyCode::KeyA) {
        delta_yaw -= camera_settings.orbit_speed;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        delta_yaw += camera_settings.orbit_speed;
    }

    // Incorporating the delta time between calls prevents this from being framerate-bound.
    delta_pitch *= time.delta_seconds();
    delta_roll *= time.delta_seconds();
    delta_yaw *= time.delta_seconds();

    // Obtain the existing pitch, yaw, and roll values from the transform.
    let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);

    // Establish the new yaw and pitch, preventing the pitch value from exceeding our limits.
    let pitch = (pitch + delta_pitch).clamp(
        camera_settings.pitch_range.start,
        camera_settings.pitch_range.end,
    );
    let roll = roll + delta_roll;
    let yaw = yaw + delta_yaw;
    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

    // Adjust the translation to maintain the correct orientation toward the orbit target.
    transform.translation = Vec3::ZERO - transform.forward() * camera_settings.orbit_distance;
}
