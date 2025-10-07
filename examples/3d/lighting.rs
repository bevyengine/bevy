//! Illustrates different lights of various types and colors, some static, some moving over
//! a simple scene.

use std::f32::consts::PI;

use bevy::{
    camera::{Exposure, PhysicalCameraParameters},
    color::palettes::css::*,
    light::CascadeShadowConfigBuilder,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Parameters(PhysicalCameraParameters {
            aperture_f_stops: 1.0,
            shutter_speed_s: 1.0 / 125.0,
            sensitivity_iso: 100.0,
            sensor_height: 0.01866,
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_exposure,
                toggle_ambient_light,
                movement,
                animate_light_direction,
            ),
        )
        .run();
}

#[derive(Resource, Default, Deref, DerefMut)]
struct Parameters(PhysicalCameraParameters);

#[derive(Component)]
struct Movable;

/// set up a simple 3D scene
fn setup(
    parameters: Res<Parameters>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    // left wall
    let mut transform = Transform::from_xyz(2.5, 2.5, 0.0);
    transform.rotate_z(PI / 2.);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(5.0, 0.15, 5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: INDIGO.into(),
            perceptual_roughness: 1.0,
            ..default()
        })),
        transform,
    ));
    // back (right) wall
    let mut transform = Transform::from_xyz(0.0, 2.5, -2.5);
    transform.rotate_x(PI / 2.);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(5.0, 0.15, 5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: INDIGO.into(),
            perceptual_roughness: 1.0,
            ..default()
        })),
        transform,
    ));

    // Bevy logo to demonstrate alpha mask shadows
    let mut transform = Transform::from_xyz(-2.2, 0.5, 1.0);
    transform.rotate_y(PI / 8.);
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(2.0, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("branding/bevy_logo_light.png")),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            ..default()
        })),
        transform,
        Movable,
    ));

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: DEEP_PINK.into(),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Movable,
    ));
    // sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5).mesh().uv(32, 18))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: LIMEGREEN.into(),
            ..default()
        })),
        Transform::from_xyz(1.5, 1.0, 1.5),
        Movable,
    ));

    // ambient light
    // ambient lights' brightnesses are measured in candela per meter square, calculable as (color * brightness)
    commands.insert_resource(AmbientLight {
        color: ORANGE_RED.into(),
        brightness: 200.0,
        ..default()
    });

    // red point light
    commands.spawn((
        PointLight {
            intensity: 100_000.0,
            color: RED.into(),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 2.0, 0.0),
        children![(
            Mesh3d(meshes.add(Sphere::new(0.1).mesh().uv(32, 18))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: RED.into(),
                emissive: LinearRgba::new(4.0, 0.0, 0.0, 0.0),
                ..default()
            })),
        )],
    ));

    // green spot light
    commands.spawn((
        SpotLight {
            intensity: 100_000.0,
            color: LIME.into(),
            shadows_enabled: true,
            inner_angle: 0.6,
            outer_angle: 0.8,
            ..default()
        },
        Transform::from_xyz(-1.0, 2.0, 0.0).looking_at(Vec3::new(-1.0, 0.0, 0.0), Vec3::Z),
        children![(
            Mesh3d(meshes.add(Capsule3d::new(0.1, 0.125))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: LIME.into(),
                emissive: LinearRgba::new(0.0, 4.0, 0.0, 0.0),
                ..default()
            })),
            Transform::from_rotation(Quat::from_rotation_x(PI / 2.0)),
        )],
    ));

    // blue point light
    commands.spawn((
        PointLight {
            intensity: 100_000.0,
            color: BLUE.into(),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 4.0, 0.0),
        children![(
            Mesh3d(meshes.add(Sphere::new(0.1).mesh().uv(32, 18))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: BLUE.into(),
                emissive: LinearRgba::new(0.0, 0.0, 713.0, 0.0),
                ..default()
            })),
        )],
    ));

    // directional 'sun' light
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        // The default cascade config is designed to handle large scenes.
        // As this example has a much smaller world, we can tighten the shadow
        // bounds for better visual quality.
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 4.0,
            maximum_distance: 10.0,
            ..default()
        }
        .build(),
    ));

    // example instructions

    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        children![
            TextSpan::new("Ambient light is on\n"),
            TextSpan(format!("Aperture: f/{:.0}\n", parameters.aperture_f_stops,)),
            TextSpan(format!(
                "Shutter speed: 1/{:.0}s\n",
                1.0 / parameters.shutter_speed_s
            )),
            TextSpan(format!(
                "Sensitivity: ISO {:.0}\n",
                parameters.sensitivity_iso
            )),
            TextSpan::new("\n\n"),
            TextSpan::new("Controls\n"),
            TextSpan::new("---------------\n"),
            TextSpan::new("Arrow keys - Move objects\n"),
            TextSpan::new("Space - Toggle ambient light\n"),
            TextSpan::new("1/2 - Decrease/Increase aperture\n"),
            TextSpan::new("3/4 - Decrease/Increase shutter speed\n"),
            TextSpan::new("5/6 - Decrease/Increase sensitivity\n"),
            TextSpan::new("R - Reset exposure"),
        ],
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Exposure::from_physical_camera(**parameters),
    ));
}

fn update_exposure(
    key_input: Res<ButtonInput<KeyCode>>,
    mut parameters: ResMut<Parameters>,
    mut exposure: Single<&mut Exposure>,
    text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    // TODO: Clamp values to a reasonable range
    let entity = *text;
    if key_input.just_pressed(KeyCode::Digit2) {
        parameters.aperture_f_stops *= 2.0;
    } else if key_input.just_pressed(KeyCode::Digit1) {
        parameters.aperture_f_stops *= 0.5;
    }
    if key_input.just_pressed(KeyCode::Digit4) {
        parameters.shutter_speed_s *= 2.0;
    } else if key_input.just_pressed(KeyCode::Digit3) {
        parameters.shutter_speed_s *= 0.5;
    }
    if key_input.just_pressed(KeyCode::Digit6) {
        parameters.sensitivity_iso += 100.0;
    } else if key_input.just_pressed(KeyCode::Digit5) {
        parameters.sensitivity_iso -= 100.0;
    }
    if key_input.just_pressed(KeyCode::KeyR) {
        *parameters = Parameters::default();
    }

    *writer.text(entity, 2) = format!("Aperture: f/{:.0}\n", parameters.aperture_f_stops);
    *writer.text(entity, 3) = format!(
        "Shutter speed: 1/{:.0}s\n",
        1.0 / parameters.shutter_speed_s
    );
    *writer.text(entity, 4) = format!("Sensitivity: ISO {:.0}\n", parameters.sensitivity_iso);

    **exposure = Exposure::from_physical_camera(**parameters);
}

fn toggle_ambient_light(
    key_input: Res<ButtonInput<KeyCode>>,
    mut ambient_light: ResMut<AmbientLight>,
    text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    if key_input.just_pressed(KeyCode::Space) {
        if ambient_light.brightness > 1. {
            ambient_light.brightness = 0.;
        } else {
            ambient_light.brightness = 200.;
        }

        let entity = *text;
        let ambient_light_state_text: &str = match ambient_light.brightness {
            0. => "off",
            _ => "on",
        };
        *writer.text(entity, 1) = format!("Ambient light is {ambient_light_state_text}\n");
    }
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() * 0.5);
    }
}

fn movement(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Movable>>,
) {
    for mut transform in &mut query {
        let mut direction = Vec3::ZERO;
        if input.pressed(KeyCode::ArrowUp) {
            direction.y += 1.0;
        }
        if input.pressed(KeyCode::ArrowDown) {
            direction.y -= 1.0;
        }
        if input.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if input.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }

        transform.translation += time.delta_secs() * 2.0 * direction;
    }
}
