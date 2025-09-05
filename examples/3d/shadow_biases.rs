//! Demonstrates how shadow biases affect shadows in a 3d scene.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{light::ShadowFilteringMethod, prelude::*};
use camera_controller::{CameraController, CameraControllerPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CameraControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                cycle_filter_methods,
                adjust_light_position,
                adjust_point_light_biases,
                toggle_light,
                adjust_directional_light_biases,
            ),
        )
        .run();
}

#[derive(Component)]
struct Lights;

/// set up a 3D scene to test shadow biases and perspective projections
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let spawn_plane_depth = 300.0f32;
    let spawn_height = 2.0;
    let sphere_radius = 0.25;

    let white_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        ..default()
    });
    let sphere_handle = meshes.add(Sphere::new(sphere_radius));

    let light_transform = Transform::from_xyz(5.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y);
    commands
        .spawn((light_transform, Visibility::default(), Lights))
        .with_children(|builder| {
            builder.spawn(PointLight {
                intensity: 0.0,
                range: spawn_plane_depth,
                color: Color::WHITE,
                shadows_enabled: true,
                ..default()
            });
            builder.spawn(DirectionalLight {
                shadows_enabled: true,
                ..default()
            });
        });

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-1.0, 1.0, 1.0).looking_at(Vec3::new(-1.0, 1.0, 0.0), Vec3::Y),
        CameraController::default(),
        ShadowFilteringMethod::Hardware2x2,
    ));

    for z_i32 in (-spawn_plane_depth as i32..=0).step_by(2) {
        commands.spawn((
            Mesh3d(sphere_handle.clone()),
            MeshMaterial3d(white_handle.clone()),
            Transform::from_xyz(
                0.0,
                if z_i32 % 4 == 0 {
                    spawn_height
                } else {
                    sphere_radius
                },
                z_i32 as f32,
            ),
        ));
    }

    // ground plane
    let plane_size = 2.0 * spawn_plane_depth;
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(plane_size, plane_size))),
        MeshMaterial3d(white_handle),
    ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.75)),
            GlobalZIndex(i32::MAX),
        ))
        .with_children(|p| {
            p.spawn(Text::default()).with_children(|p| {
                p.spawn(TextSpan::new("Controls:\n"));
                p.spawn(TextSpan::new("R / Z - reset biases to default / zero\n"));
                p.spawn(TextSpan::new(
                    "L     - switch between directional and point lights [",
                ));
                p.spawn(TextSpan::new("DirectionalLight"));
                p.spawn(TextSpan::new("]\n"));
                p.spawn(TextSpan::new(
                    "F     - switch directional light filter methods [",
                ));
                p.spawn(TextSpan::new("Hardware2x2"));
                p.spawn(TextSpan::new("]\n"));
                p.spawn(TextSpan::new("1/2   - change point light depth bias ["));
                p.spawn(TextSpan::new("0.00"));
                p.spawn(TextSpan::new("]\n"));
                p.spawn(TextSpan::new("3/4   - change point light normal bias ["));
                p.spawn(TextSpan::new("0.0"));
                p.spawn(TextSpan::new("]\n"));
                p.spawn(TextSpan::new("5/6   - change direction light depth bias ["));
                p.spawn(TextSpan::new("0.00"));
                p.spawn(TextSpan::new("]\n"));
                p.spawn(TextSpan::new(
                    "7/8   - change direction light normal bias [",
                ));
                p.spawn(TextSpan::new("0.0"));
                p.spawn(TextSpan::new("]\n"));
                p.spawn(TextSpan::new(
                    "left/right/up/down/pgup/pgdown - adjust light position (looking at 0,0,0) [",
                ));
                p.spawn(TextSpan(format!("{:.1},", light_transform.translation.x)));
                p.spawn(TextSpan(format!(" {:.1},", light_transform.translation.y)));
                p.spawn(TextSpan(format!(" {:.1}", light_transform.translation.z)));
                p.spawn(TextSpan::new("]\n"));
            });
        });
}

fn toggle_light(
    input: Res<ButtonInput<KeyCode>>,
    mut point_lights: Query<&mut PointLight>,
    mut directional_lights: Query<&mut DirectionalLight>,
    example_text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    if input.just_pressed(KeyCode::KeyL) {
        for mut light in &mut point_lights {
            light.intensity = if light.intensity == 0.0 {
                *writer.text(*example_text, 4) = "PointLight".to_string();
                light_consts::lumens::VERY_LARGE_CINEMA_LIGHT
            } else {
                0.0
            };
        }
        for mut light in &mut directional_lights {
            light.illuminance = if light.illuminance == 0.0 {
                *writer.text(*example_text, 4) = "DirectionalLight".to_string();
                light_consts::lux::AMBIENT_DAYLIGHT
            } else {
                0.0
            };
        }
    }
}

fn adjust_light_position(
    input: Res<ButtonInput<KeyCode>>,
    mut lights: Query<&mut Transform, With<Lights>>,
    example_text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    let mut offset = Vec3::ZERO;
    if input.just_pressed(KeyCode::ArrowLeft) {
        offset.x -= 1.0;
    }
    if input.just_pressed(KeyCode::ArrowRight) {
        offset.x += 1.0;
    }
    if input.just_pressed(KeyCode::ArrowUp) {
        offset.z -= 1.0;
    }
    if input.just_pressed(KeyCode::ArrowDown) {
        offset.z += 1.0;
    }
    if input.just_pressed(KeyCode::PageDown) {
        offset.y -= 1.0;
    }
    if input.just_pressed(KeyCode::PageUp) {
        offset.y += 1.0;
    }
    if offset != Vec3::ZERO {
        let example_text = *example_text;
        for mut light in &mut lights {
            light.translation += offset;
            light.look_at(Vec3::ZERO, Vec3::Y);
            *writer.text(example_text, 22) = format!("{:.1},", light.translation.x);
            *writer.text(example_text, 23) = format!(" {:.1},", light.translation.y);
            *writer.text(example_text, 24) = format!(" {:.1}", light.translation.z);
        }
    }
}

fn cycle_filter_methods(
    input: Res<ButtonInput<KeyCode>>,
    mut filter_methods: Query<&mut ShadowFilteringMethod>,
    example_text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    if input.just_pressed(KeyCode::KeyF) {
        for mut filter_method in &mut filter_methods {
            let filter_method_string;
            *filter_method = match *filter_method {
                ShadowFilteringMethod::Hardware2x2 => {
                    filter_method_string = "Gaussian".to_string();
                    ShadowFilteringMethod::Gaussian
                }
                ShadowFilteringMethod::Gaussian => {
                    filter_method_string = "Temporal".to_string();
                    ShadowFilteringMethod::Temporal
                }
                ShadowFilteringMethod::Temporal => {
                    filter_method_string = "Hardware2x2".to_string();
                    ShadowFilteringMethod::Hardware2x2
                }
            };
            *writer.text(*example_text, 7) = filter_method_string;
        }
    }
}

fn adjust_point_light_biases(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut PointLight>,
    example_text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    let depth_bias_step_size = 0.01;
    let normal_bias_step_size = 0.1;
    for mut light in &mut query {
        if input.just_pressed(KeyCode::Digit1) {
            light.shadow_depth_bias -= depth_bias_step_size;
        }
        if input.just_pressed(KeyCode::Digit2) {
            light.shadow_depth_bias += depth_bias_step_size;
        }
        if input.just_pressed(KeyCode::Digit3) {
            light.shadow_normal_bias -= normal_bias_step_size;
        }
        if input.just_pressed(KeyCode::Digit4) {
            light.shadow_normal_bias += normal_bias_step_size;
        }
        if input.just_pressed(KeyCode::KeyR) {
            light.shadow_depth_bias = PointLight::DEFAULT_SHADOW_DEPTH_BIAS;
            light.shadow_normal_bias = PointLight::DEFAULT_SHADOW_NORMAL_BIAS;
        }
        if input.just_pressed(KeyCode::KeyZ) {
            light.shadow_depth_bias = 0.0;
            light.shadow_normal_bias = 0.0;
        }

        *writer.text(*example_text, 10) = format!("{:.2}", light.shadow_depth_bias);
        *writer.text(*example_text, 13) = format!("{:.1}", light.shadow_normal_bias);
    }
}

fn adjust_directional_light_biases(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut DirectionalLight>,
    example_text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    let depth_bias_step_size = 0.01;
    let normal_bias_step_size = 0.1;
    for mut light in &mut query {
        if input.just_pressed(KeyCode::Digit5) {
            light.shadow_depth_bias -= depth_bias_step_size;
        }
        if input.just_pressed(KeyCode::Digit6) {
            light.shadow_depth_bias += depth_bias_step_size;
        }
        if input.just_pressed(KeyCode::Digit7) {
            light.shadow_normal_bias -= normal_bias_step_size;
        }
        if input.just_pressed(KeyCode::Digit8) {
            light.shadow_normal_bias += normal_bias_step_size;
        }
        if input.just_pressed(KeyCode::KeyR) {
            light.shadow_depth_bias = DirectionalLight::DEFAULT_SHADOW_DEPTH_BIAS;
            light.shadow_normal_bias = DirectionalLight::DEFAULT_SHADOW_NORMAL_BIAS;
        }
        if input.just_pressed(KeyCode::KeyZ) {
            light.shadow_depth_bias = 0.0;
            light.shadow_normal_bias = 0.0;
        }

        *writer.text(*example_text, 16) = format!("{:.2}", light.shadow_depth_bias);
        *writer.text(*example_text, 19) = format!("{:.1}", light.shadow_normal_bias);
    }
}
