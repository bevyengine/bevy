//! Shows how to zoom and orbit an orthographic projection camera.

use std::ops::Range;

use bevy::{input::mouse::AccumulatedMouseScroll, prelude::*, render::camera::ScalingMode};

#[derive(Debug, Default, Reflect, Resource)]
struct CameraSettings {
    // Multiply keyboard inputs by this factor
    pub orbit_speed: f32,
    // Clamp fixed vertical scale to this range
    pub zoom_range: Range<f32>,
    // Multiply mouse wheel movements by this factor
    pub zoom_speed: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<CameraSettings>()
        .add_systems(Startup, (setup, instructions))
        .add_systems(Update, camera_controls)
        .register_type::<CameraSettings>()
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut camera_settings: ResMut<CameraSettings>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    camera_settings.orbit_speed = 0.02;
    camera_settings.zoom_range = 5.0..50.0;
    camera_settings.zoom_speed = 1.0;

    // Find the middle of the zoom range
    let initial_scale = (camera_settings.zoom_range.start + camera_settings.zoom_range.end) / 2.0;

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
}

fn instructions(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Start,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Start,
                width: Val::Percent(100.),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Scroll mouse wheel to zoom in/out",
                TextStyle::default(),
            ));
            parent.spawn(TextBundle::from_section(
                "A or D to orbit left or right",
                TextStyle::default(),
            ));
        });
}

fn camera_controls(
    mut camera: Query<(&mut Projection, &mut Transform), With<Camera>>,
    camera_settings: Res<CameraSettings>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_wheel_input: Res<AccumulatedMouseScroll>,
) {
    let mut delta_orbit = 0.0;
    if keyboard_input.pressed(KeyCode::KeyA) {
        // Orbit left
        delta_orbit -= camera_settings.orbit_speed;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        // Orbit right
        delta_orbit += camera_settings.orbit_speed;
    }

    let (mut projection, mut transform) = camera.single_mut();

    if delta_orbit != 0.0 {
        // Orbit the camera around a fixed point, facing its center.
        transform.translate_around(Vec3::ZERO, Quat::from_axis_angle(Vec3::Y, delta_orbit));
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }

    let Projection::Orthographic(orthographic) = &mut *projection else {
        panic!(
            "This kind of scaling only works with cameras which have an orthographic projection."
        );
    };
    // Get the current scaling_mode value to allow clamping the new value to our zoom range.
    let ScalingMode::FixedVertical(current) = orthographic.scaling_mode else {
        return;
    };
    // Set a new ScalingMode, clamped to a limited range.
    let zoom_level = (current + camera_settings.zoom_speed * mouse_wheel_input.delta.y).clamp(
        camera_settings.zoom_range.start,
        camera_settings.zoom_range.end,
    );
    orthographic.scaling_mode = ScalingMode::FixedVertical(zoom_level);
}
