//! Shows how to zoom and orbit orthographic and perspective projection cameras.

use std::{
    f32::consts::{FRAC_PI_2, PI},
    ops::Range,
};

use bevy::{input::mouse::AccumulatedMouseScroll, prelude::*, render::camera::ScalingMode};

#[derive(Debug, Default, Resource)]
struct CameraSettings {
    pub orbit_distance: f32,
    // Multiply keyboard inputs by this factor
    pub orbit_speed: f32,
    // Clamp fixed vertical scale to this range
    pub orthographic_zoom_range: Range<f32>,
    // Multiply mouse wheel inputs by this factor
    pub orthographic_zoom_speed: f32,
    // Clamp field of view to this range
    pub perspective_zoom_range: Range<f32>,
    // Multiply mouse wheel inputs by this factor
    pub perspective_zoom_speed: f32,
    // Clamp pitch to this range
    pub pitch_range: Range<f32>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<CameraSettings>()
        .add_systems(Startup, (setup, instructions))
        .add_systems(Update, (orbit, switch_projection, zoom))
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut camera_settings: ResMut<CameraSettings>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Perspective projections use field of view, expressed in radians. We would
    // normally not set it to more than π, which represents a 180° FOV.
    let min_fov = PI / 5.;
    let max_fov = PI - 0.2;

    // In orthographic projections, we specify sizes in world units. The below values
    // are very roughly similar to the above FOV settings, in terms of how "far away"
    // the subject will appear when used with FixedVertical scaling mode.
    let min_zoom = 5.0;
    let max_zoom = 150.0;

    // Limiting pitch stops some unexpected rotation past 90° up or down.
    let pitch_limit = FRAC_PI_2 - 0.01;

    camera_settings.orbit_distance = 10.0;
    camera_settings.orbit_speed = 1.0;
    camera_settings.orthographic_zoom_range = min_zoom..max_zoom;
    camera_settings.orthographic_zoom_speed = 1.0;
    camera_settings.perspective_zoom_range = min_fov..max_fov;
    // Changes in FOV are much more noticeable due to its limited range in radians
    camera_settings.perspective_zoom_speed = 0.05;
    camera_settings.pitch_range = -pitch_limit..pitch_limit;

    commands.spawn((
        Name::new("Camera"),
        Camera3dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical(
                    camera_settings.orthographic_zoom_range.start,
                ),
                ..default()
            }
            .into(),
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
                "Scroll mouse wheel to zoom in/out",
                TextStyle::default(),
            ));
            parent.spawn(TextBundle::from_section(
                "W or S: pitch",
                TextStyle::default(),
            ));
            parent.spawn(TextBundle::from_section(
                "A or D: yaw",
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
    let mut delta_yaw = 0.0;

    if keyboard_input.pressed(KeyCode::KeyW) {
        delta_pitch += camera_settings.orbit_speed;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        delta_yaw -= camera_settings.orbit_speed;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        delta_pitch -= camera_settings.orbit_speed;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        delta_yaw += camera_settings.orbit_speed;
    }

    // Incorporating the delta time between calls prevents this from being framerate-bound.
    delta_pitch *= time.delta_seconds();
    delta_yaw *= time.delta_seconds();

    // Obtain the existing pitch, yaw, and roll values from the transform.
    let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);

    // Establish the new yaw and pitch, preventing the pitch value from exceeding our limits.
    let pitch = (pitch + delta_pitch).clamp(
        camera_settings.pitch_range.start,
        camera_settings.pitch_range.end,
    );
    let yaw = yaw + delta_yaw;
    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

    // Adjust the translation to maintain the correct orientation toward the orbit target.
    transform.translation = Vec3::ZERO - transform.forward() * camera_settings.orbit_distance;
}

fn switch_projection(
    mut camera: Query<&mut Projection, With<Camera>>,
    camera_settings: Res<CameraSettings>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let mut projection = camera.single_mut();

    if keyboard_input.just_pressed(KeyCode::Space) {
        // Switch projection type
        *projection = match *projection {
            Projection::Orthographic(_) => Projection::Perspective(PerspectiveProjection {
                fov: camera_settings.perspective_zoom_range.start,
                ..default()
            }),
            Projection::Perspective(_) => Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical(
                    camera_settings.orthographic_zoom_range.start,
                ),
                ..default()
            }),
        }
    }
}

fn zoom(
    mut camera: Query<&mut Projection, With<Camera>>,
    camera_settings: Res<CameraSettings>,
    mouse_wheel_input: Res<AccumulatedMouseScroll>,
) {
    let mut projection = camera.single_mut();

    // Usually, you won't need to handle both types of projection. This is by way of demonstration.
    match &mut *projection {
        Projection::Orthographic(orthographic) => {
            // Get the current scaling_mode value to allow clamping the new value to our zoom range.
            let ScalingMode::FixedVertical(current) = orthographic.scaling_mode else {
                return;
            };
            // Set a new ScalingMode, clamped to a limited range.
            let zoom_level = (current
                + camera_settings.orthographic_zoom_speed * mouse_wheel_input.delta.y)
                .clamp(
                    camera_settings.orthographic_zoom_range.start,
                    camera_settings.orthographic_zoom_range.end,
                );
            orthographic.scaling_mode = ScalingMode::FixedVertical(zoom_level);
        }
        Projection::Perspective(perspective) => {
            // Adjust the field of view, but keep it within our stated range. Note that we divide
            // by an arbitrary factor here to prevent the perspective FOV change from seeming much
            // faster than its orthographic equivalent.
            perspective.fov = (perspective.fov
                + camera_settings.perspective_zoom_speed * mouse_wheel_input.delta.y)
                .clamp(
                    camera_settings.perspective_zoom_range.start,
                    camera_settings.perspective_zoom_range.end,
                );
        }
    }
}
