//! Demonstrates how shadow biases affect shadows in a 3d scene.

use std::f32::consts::PI;

use bevy::{input::mouse::MouseMotion, pbr::ShadowFilteringMethod, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                cycle_filter_methods,
                adjust_light_position,
                adjust_point_light_biases,
                toggle_light,
                adjust_directional_light_biases,
                camera_controller,
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
    let sphere_handle = meshes.add(
        Mesh::try_from(shape::Icosphere {
            radius: sphere_radius,
            ..default()
        })
        .unwrap(),
    );

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
                    illuminance: 100000.0,
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
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(2.0 * spawn_plane_depth)),
        material: white_handle,
        ..default()
    });

    let style = TextStyle {
        font_size: 20.,
        ..default()
    };
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                padding: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            z_index: ZIndex::Global(i32::MAX),
            background_color: Color::BLACK.with_a(0.75).into(),
            ..default()
        })
        .with_children(|c| {
            c.spawn(TextBundle::from_sections([
                TextSection::new("Controls:\n", style.clone()),
                TextSection::new("WSAD  - forward/back/strafe left/right\n", style.clone()),
                TextSection::new("E / Q - up / down\n", style.clone()),
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
                example_text.single_mut().sections[5].value = "PointLight".to_string();
                100000000.0
            } else {
                0.0
            };
        }
        for mut light in &mut directional_lights {
            light.illuminance = if light.illuminance == 0.0 {
                example_text.single_mut().sections[5].value = "DirectionalLight".to_string();
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
            example_text.sections[23].value = format!("{:.1},", light.translation.x);
            example_text.sections[24].value = format!(" {:.1},", light.translation.y);
            example_text.sections[25].value = format!(" {:.1}", light.translation.z);
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
                    filter_method_string = "Castano13".to_string();
                    ShadowFilteringMethod::Castano13
                }
                ShadowFilteringMethod::Castano13 => {
                    filter_method_string = "Jimenez14".to_string();
                    ShadowFilteringMethod::Jimenez14
                }
                ShadowFilteringMethod::Jimenez14 => {
                    filter_method_string = "Hardware2x2".to_string();
                    ShadowFilteringMethod::Hardware2x2
                }
            };
            example_text.single_mut().sections[8].value = filter_method_string;
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

        example_text.single_mut().sections[11].value = format!("{:.2}", light.shadow_depth_bias);
        example_text.single_mut().sections[14].value = format!("{:.1}", light.shadow_normal_bias);
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

        example_text.single_mut().sections[17].value = format!("{:.2}", light.shadow_depth_bias);
        example_text.single_mut().sections[20].value = format!("{:.1}", light.shadow_normal_bias);
    }
}

#[derive(Component)]
struct CameraController {
    pub enabled: bool,
    pub sensitivity: f32,
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_run: KeyCode,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub friction: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub velocity: Vec3,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: 0.5,
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_up: KeyCode::KeyE,
            key_down: KeyCode::KeyQ,
            key_run: KeyCode::ShiftLeft,
            walk_speed: 10.0,
            run_speed: 30.0,
            friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

fn camera_controller(
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    let dt = time.delta_seconds();

    // Handle mouse input
    let mut mouse_delta = Vec2::ZERO;
    for mouse_event in mouse_events.read() {
        mouse_delta += mouse_event.delta;
    }

    for (mut transform, mut options) in &mut query {
        if !options.enabled {
            continue;
        }

        // Handle key input
        let mut axis_input = Vec3::ZERO;
        if key_input.pressed(options.key_forward) {
            axis_input.z += 1.0;
        }
        if key_input.pressed(options.key_back) {
            axis_input.z -= 1.0;
        }
        if key_input.pressed(options.key_right) {
            axis_input.x += 1.0;
        }
        if key_input.pressed(options.key_left) {
            axis_input.x -= 1.0;
        }
        if key_input.pressed(options.key_up) {
            axis_input.y += 1.0;
        }
        if key_input.pressed(options.key_down) {
            axis_input.y -= 1.0;
        }

        // Apply movement update
        if axis_input != Vec3::ZERO {
            let max_speed = if key_input.pressed(options.key_run) {
                options.run_speed
            } else {
                options.walk_speed
            };
            options.velocity = axis_input.normalize() * max_speed;
        } else {
            let friction = options.friction.clamp(0.0, 1.0);
            options.velocity *= 1.0 - friction;
            if options.velocity.length_squared() < 1e-6 {
                options.velocity = Vec3::ZERO;
            }
        }
        let forward = transform.forward();
        let right = transform.right();
        transform.translation += options.velocity.x * dt * right
            + options.velocity.y * dt * Vec3::Y
            + options.velocity.z * dt * forward;

        if mouse_delta != Vec2::ZERO {
            // Apply look update
            options.pitch = (options.pitch - mouse_delta.y * 0.5 * options.sensitivity * dt)
                .clamp(-PI / 2., PI / 2.);
            options.yaw -= mouse_delta.x * options.sensitivity * dt;
            transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, options.yaw, options.pitch);
        }
    }
}
