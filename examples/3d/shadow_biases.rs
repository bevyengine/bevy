//! Demonstrates how shadow biases affect shadows in a 3d scene.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{pbr::ShadowFilteringMethod, prelude::*};
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
        .spawn((
            SpatialBundle {
                transform: light_transform,
                ..default()
            },
            Lights,
        ))
        .with_children(|builder| {
            builder.spawn(PointLightBundle {
                point_light: PointLight {
                    intensity: 0.0,
                    range: spawn_plane_depth,
                    color: Color::WHITE,
                    shadow_depth_bias: 0.0,
                    shadow_normal_bias: 0.0,
                    shadows_enabled: true,
                    ..default()
                },
                ..default()
            });
            builder.spawn(DirectionalLightBundle {
                directional_light: DirectionalLight {
                    shadow_depth_bias: 0.0,
                    shadow_normal_bias: 0.0,
                    shadows_enabled: true,
                    ..default()
                },
                ..default()
            });
        });

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-1.0, 1.0, 1.0)
                .looking_at(Vec3::new(-1.0, 1.0, 0.0), Vec3::Y),
            ..default()
        },
        CameraController::default(),
        ShadowFilteringMethod::Hardware2x2,
    ));

    for z_i32 in (-spawn_plane_depth as i32..=0).step_by(2) {
        commands.spawn(PbrBundle {
            mesh: sphere_handle.clone(),
            material: white_handle.clone(),
            transform: Transform::from_xyz(
                0.0,
                if z_i32 % 4 == 0 {
                    spawn_height
                } else {
                    sphere_radius
                },
                z_i32 as f32,
            ),
            ..default()
        });
    }

    // ground plane
    let plane_size = 2.0 * spawn_plane_depth;
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(plane_size, plane_size)),
        material: white_handle,
        ..default()
    });

    let style = TextStyle::default();

    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                padding: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            z_index: ZIndex::Global(i32::MAX),
            background_color: Color::BLACK.with_alpha(0.75).into(),
            ..default()
        })
        .with_children(|c| {
            c.spawn(TextBundle::from_sections([
                TextSection::new("Controls:\n", style.clone()),
                TextSection::new("R / Z - reset biases to default / zero\n", style.clone()),
                TextSection::new(
                    "L     - switch between directional and point lights [",
                    style.clone(),
                ),
                TextSection::new("DirectionalLight", style.clone()),
                TextSection::new("]\n", style.clone()),
                TextSection::new(
                    "F     - switch directional light filter methods [",
                    style.clone(),
                ),
                TextSection::new("Hardware2x2", style.clone()),
                TextSection::new("]\n", style.clone()),
                TextSection::new("1/2   - change point light depth bias [", style.clone()),
                TextSection::new("0.00", style.clone()),
                TextSection::new("]\n", style.clone()),
                TextSection::new("3/4   - change point light normal bias [", style.clone()),
                TextSection::new("0.0", style.clone()),
                TextSection::new("]\n", style.clone()),
                TextSection::new("5/6   - change direction light depth bias [", style.clone()),
                TextSection::new("0.00", style.clone()),
                TextSection::new("]\n", style.clone()),
                TextSection::new(
                    "7/8   - change direction light normal bias [",
                    style.clone(),
                ),
                TextSection::new("0.0", style.clone()),
                TextSection::new("]\n", style.clone()),
                TextSection::new(
                    "left/right/up/down/pgup/pgdown - adjust light position (looking at 0,0,0) [",
                    style.clone(),
                ),
                TextSection::new(
                    format!("{:.1},", light_transform.translation.x),
                    style.clone(),
                ),
                TextSection::new(
                    format!(" {:.1},", light_transform.translation.y),
                    style.clone(),
                ),
                TextSection::new(
                    format!(" {:.1}", light_transform.translation.z),
                    style.clone(),
                ),
                TextSection::new("]\n", style.clone()),
            ]));
        });
}

fn toggle_light(
    input: Res<ButtonInput<KeyCode>>,
    mut point_lights: Query<&mut PointLight>,
    mut directional_lights: Query<&mut DirectionalLight>,
    mut example_text: Query<&mut Text>,
) {
    if input.just_pressed(KeyCode::KeyL) {
        for mut light in &mut point_lights {
            light.intensity = if light.intensity == 0.0 {
                example_text.single_mut().sections[3].value = "PointLight".to_string();
                100000000.0
            } else {
                0.0
            };
        }
        for mut light in &mut directional_lights {
            light.illuminance = if light.illuminance == 0.0 {
                example_text.single_mut().sections[3].value = "DirectionalLight".to_string();
                100000.0
            } else {
                0.0
            };
        }
    }
}

fn adjust_light_position(
    input: Res<ButtonInput<KeyCode>>,
    mut lights: Query<&mut Transform, With<Lights>>,
    mut example_text: Query<&mut Text>,
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
        let mut example_text = example_text.single_mut();
        for mut light in &mut lights {
            light.translation += offset;
            light.look_at(Vec3::ZERO, Vec3::Y);
            example_text.sections[21].value = format!("{:.1},", light.translation.x);
            example_text.sections[22].value = format!(" {:.1},", light.translation.y);
            example_text.sections[23].value = format!(" {:.1}", light.translation.z);
        }
    }
}

fn cycle_filter_methods(
    input: Res<ButtonInput<KeyCode>>,
    mut filter_methods: Query<&mut ShadowFilteringMethod>,
    mut example_text: Query<&mut Text>,
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
            example_text.single_mut().sections[6].value = filter_method_string;
        }
    }
}

fn adjust_point_light_biases(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut PointLight>,
    mut example_text: Query<&mut Text>,
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

        example_text.single_mut().sections[9].value = format!("{:.2}", light.shadow_depth_bias);
        example_text.single_mut().sections[12].value = format!("{:.1}", light.shadow_normal_bias);
    }
}

fn adjust_directional_light_biases(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut DirectionalLight>,
    mut example_text: Query<&mut Text>,
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

        example_text.single_mut().sections[15].value = format!("{:.2}", light.shadow_depth_bias);
        example_text.single_mut().sections[18].value = format!("{:.1}", light.shadow_normal_bias);
    }
}
