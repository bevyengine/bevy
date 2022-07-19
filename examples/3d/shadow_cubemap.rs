use bevy::{input::mouse::MouseMotion, pbr::NotShadowCaster, prelude::*};

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(camera_controller)
        .add_system(text_update_system)
        .run();
}

#[derive(Component)]
struct RightHandedLookDirection;
#[derive(Component)]
struct LeftHandedLookDirection;

/// A test for shadow cubemaps. View the cubemap faces in RenderDoc/Xcode.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    // spawn the test scene, and particularly the light, offset from the origin to catch errors
    // with the shadow mapping due to translations
    let parent = commands
        .spawn_bundle(SpatialBundle {
            transform: Transform::from_translation(Vec3::ONE),
            ..default()
        })
        .with_children(|parent| {
            // a point light with shadows at the local origin
            parent
                .spawn_bundle(PointLightBundle {
                    point_light: PointLight {
                        intensity: 50.0,
                        shadows_enabled: true,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|builder| {
                    builder
                        .spawn_bundle(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Icosphere::default())),
                            material: materials.add(Color::FUCHSIA.into()),
                            transform: Transform::from_scale(Vec3::splat(0.05)),
                            ..Default::default()
                        })
                        .insert(NotShadowCaster);
                });
        })
        .id();

    scene_spawner.spawn_as_child(
        asset_server.load("models/left-handed-cubemap-test.gltf#Scene0"),
        parent,
    );

    // camera
    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_translation(0.9 * Vec3::ONE),
            ..default()
        })
        .insert(CameraController::default());

    // UI displaying the look direction as text in the top-left of the screen
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                size: Size::new(Val::Auto, Val::Percent(100.0)),
                ..default()
            },
            color: Color::NONE.into(),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(NodeBundle {
                color: Color::NONE.into(),
                ..default()
            });
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::ColumnReverse,
                        ..default()
                    },
                    color: Color::rgba(0.5, 0.5, 0.5, 0.5).into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexEnd,
                                ..default()
                            },
                            // Use `Text` directly
                            text: Text {
                                // Construct a `Vec` of `TextSection`s
                                sections: vec![
                                    TextSection {
                                        value: "Right-handed look-direction:".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                            font_size: 32.0,
                                            color: Color::BLACK,
                                        },
                                    },
                                    TextSection {
                                        value: "".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                                            font_size: 32.0,
                                            color: Color::RED,
                                        },
                                    },
                                    TextSection {
                                        value: "".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                                            font_size: 32.0,
                                            color: Color::GREEN,
                                        },
                                    },
                                    TextSection {
                                        value: "".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                                            font_size: 32.0,
                                            color: Color::BLUE,
                                        },
                                    },
                                ],
                                ..default()
                            },
                            ..default()
                        })
                        .insert(RightHandedLookDirection);
                    parent
                        .spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexEnd,
                                ..default()
                            },
                            // Use `Text` directly
                            text: Text {
                                // Construct a `Vec` of `TextSection`s
                                sections: vec![
                                    TextSection {
                                        value: "Left-handed look-direction:".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                            font_size: 32.0,
                                            color: Color::BLACK,
                                        },
                                    },
                                    TextSection {
                                        value: "".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                                            font_size: 32.0,
                                            color: Color::RED,
                                        },
                                    },
                                    TextSection {
                                        value: "".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                                            font_size: 32.0,
                                            color: Color::GREEN,
                                        },
                                    },
                                    TextSection {
                                        value: "".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                                            font_size: 32.0,
                                            color: Color::BLUE,
                                        },
                                    },
                                ],
                                ..default()
                            },
                            ..default()
                        })
                        .insert(LeftHandedLookDirection);
                });
        });
}

fn text_update_system(
    camera: Query<&Transform, With<Camera3d>>,
    mut right_handed: Query<
        &mut Text,
        (
            With<RightHandedLookDirection>,
            Without<LeftHandedLookDirection>,
        ),
    >,
    mut left_handed: Query<
        &mut Text,
        (
            Without<RightHandedLookDirection>,
            With<LeftHandedLookDirection>,
        ),
    >,
) {
    let forward = camera.single().forward();
    let mut right_handed_text = right_handed.single_mut();
    right_handed_text.sections[1].value = format!(" x: {:6.3}", forward.x);
    right_handed_text.sections[2].value = format!(", y: {:6.3}", forward.y);
    right_handed_text.sections[3].value = format!(", z: {:6.3}", forward.z);
    let mut left_handed_text = left_handed.single_mut();
    left_handed_text.sections[1].value = format!(" x: {:6.3}", forward.x);
    left_handed_text.sections[2].value = format!(", y: {:6.3}", forward.y);
    left_handed_text.sections[3].value = format!(", z: {:6.3}", -forward.z);
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
            key_forward: KeyCode::W,
            key_back: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::E,
            key_down: KeyCode::Q,
            key_run: KeyCode::LShift,
            walk_speed: 2.0,
            run_speed: 6.0,
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
    key_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    let dt = time.delta_seconds();

    // Handle mouse input
    let mut mouse_delta = Vec2::ZERO;
    for mouse_event in mouse_events.iter() {
        mouse_delta += mouse_event.delta;
    }

    for (mut transform, mut options) in query.iter_mut() {
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
            let (pitch, yaw) = (
                (options.pitch - mouse_delta.y * 0.5 * options.sensitivity * dt).clamp(
                    -0.99 * std::f32::consts::FRAC_PI_2,
                    0.99 * std::f32::consts::FRAC_PI_2,
                ),
                options.yaw - mouse_delta.x * options.sensitivity * dt,
            );
            transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, yaw, pitch);
            options.pitch = pitch;
            options.yaw = yaw;
        }
    }
}
