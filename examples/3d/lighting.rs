use bevy::{
    app::AppExit,
    diagnostic::{
        Diagnostic, DiagnosticId, Diagnostics, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
    },
    input::mouse::MouseMotion,
    pbr::{ClusterConfig, ClusterFarZMode, ClusterZConfig, Clusters, VisiblePointLights},
    prelude::*,
    window::{PresentMode, WindowMode},
};

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            width: 1280.0,
            height: 720.0,
            present_mode: PresentMode::Mailbox,
            // mode: WindowMode::BorderlessFullscreen,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_system(camera_controller)
        .add_system(cluster_style)
        .add_system(animate_light_direction)
        .add_startup_system(setup_cluster_diagnostics)
        .add_system(cluster_diagnostics)
        .run();
}

#[derive(Component)]
struct Movable;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ground plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 1000.0 })),
        material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            ..Default::default()
        }),
        ..Default::default()
    });

    // left wall
    let mut transform = Transform::from_xyz(2.5, 2.5, 0.0);
    transform.rotate(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(5.0, 0.15, 5.0))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::INDIGO,
            perceptual_roughness: 1.0,
            ..Default::default()
        }),
        ..Default::default()
    });
    // back (right) wall
    let mut transform = Transform::from_xyz(0.0, 2.5, -2.5);
    transform.rotate(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(5.0, 0.15, 5.0))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::INDIGO,
            perceptual_roughness: 1.0,
            ..Default::default()
        }),
        ..Default::default()
    });

    // cube
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(StandardMaterial {
                base_color: Color::PINK,
                ..Default::default()
            }),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..Default::default()
        })
        .insert(Movable);
    // sphere
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.5,
                ..Default::default()
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::LIME_GREEN,
                ..Default::default()
            }),
            transform: Transform::from_xyz(1.5, 1.0, 1.5),
            ..Default::default()
        })
        .insert(Movable);

    // ambient light
    commands.insert_resource(AmbientLight {
        color: Color::ORANGE_RED,
        brightness: 0.02,
    });

    // red point light
    commands
        .spawn_bundle(PointLightBundle {
            // transform: Transform::from_xyz(5.0, 8.0, 2.0),
            transform: Transform::from_xyz(1.0, 2.0, 0.0),
            point_light: PointLight {
                intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
                color: Color::RED,
                shadows_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.1,
                    ..Default::default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::RED,
                    emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                    ..Default::default()
                }),
                ..Default::default()
            });
        });

    // green point light
    commands
        .spawn_bundle(PointLightBundle {
            // transform: Transform::from_xyz(5.0, 8.0, 2.0),
            transform: Transform::from_xyz(-1.0, 2.0, 0.0),
            point_light: PointLight {
                intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
                color: Color::GREEN,
                shadows_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.1,
                    ..Default::default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::GREEN,
                    emissive: Color::rgba_linear(0.0, 100.0, 0.0, 0.0),
                    ..Default::default()
                }),
                ..Default::default()
            });
        });

    // blue point light
    commands
        .spawn_bundle(PointLightBundle {
            // transform: Transform::from_xyz(5.0, 8.0, 2.0),
            transform: Transform::from_xyz(0.0, 4.0, 0.0),
            point_light: PointLight {
                intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
                color: Color::BLUE,
                shadows_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.1,
                    ..Default::default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::BLUE,
                    emissive: Color::rgba_linear(0.0, 0.0, 100.0, 0.0),
                    ..Default::default()
                }),
                ..Default::default()
            });
        });

    // // directional 'sun' light
    // const HALF_SIZE: f32 = 10.0;
    // commands.spawn_bundle(DirectionalLightBundle {
    //     directional_light: DirectionalLight {
    //         // Configure the projection to better fit the scene
    //         shadow_projection: OrthographicProjection {
    //             left: -HALF_SIZE,
    //             right: HALF_SIZE,
    //             bottom: -HALF_SIZE,
    //             top: HALF_SIZE,
    //             near: -10.0 * HALF_SIZE,
    //             far: 10.0 * HALF_SIZE,
    //             ..Default::default()
    //         },
    //         shadows_enabled: true,
    //         ..Default::default()
    //     },
    //     transform: Transform {
    //         translation: Vec3::new(0.0, 2.0, 0.0),
    //         rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
    //         ..Default::default()
    //     },
    //     ..Default::default()
    // });

    for x in 0..6 {
        for y in 0..6 {
            for z in 0..6 {
                // red point light
                commands
                    .spawn_bundle(PointLightBundle {
                        transform: Transform::from_translation(Vec3::new(
                            x as f32 * 10.0 - 25.0,
                            y as f32 * 10.0 + 10.0,
                            z as f32 * 10.0 - 25.0,
                        )),
                        point_light: PointLight {
                            intensity: 500.0,
                            range: f32::sqrt(500.0 * 10.0 / (4.0 * std::f32::consts::PI)),
                            color: Color::RED,
                            shadows_enabled: false,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|builder| {
                        builder.spawn_bundle(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::UVSphere {
                                radius: 1.0,
                                ..Default::default()
                            })),
                            material: materials.add(StandardMaterial {
                                base_color: Color::RED,
                                emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                                ..Default::default()
                            }),
                            ..Default::default()
                        });
                    });
            }
        }
    }

    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            // transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            transform: Transform::from_xyz(-25.0, 65.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert(CameraController::default());
    // commands.spawn_bundle(OrthographicCameraBundle::new_3d()).insert(CameraController::default());
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in query.iter_mut() {
        transform.rotate(Quat::from_rotation_y(time.delta_seconds() * 0.5));
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
            key_forward: KeyCode::W,
            key_back: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::E,
            key_down: KeyCode::Q,
            key_run: KeyCode::LShift,
            walk_speed: 10.0,
            run_speed: 30.0,
            friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

fn cluster_style(
    mut q: Query<&mut ClusterConfig>,
    key_input: Res<Input<KeyCode>>,
    mut current: Local<usize>,
) {
    let configs = vec![
        ClusterConfig::Single,
        ClusterConfig::FixedZ {
            total: 4096,
            z_slices: 24,
            z_config: ClusterZConfig {
                first_slice_depth: 5.0,
                far_z_mode: ClusterFarZMode::CameraFarPlane,
            },
        },
        ClusterConfig::FixedZ {
            total: 4096,
            z_slices: 24,
            z_config: ClusterZConfig {
                first_slice_depth: 5.0,
                far_z_mode: ClusterFarZMode::MaxLightRange,
            },
        },
        ClusterConfig::FixedZ {
            total: 1024,
            z_slices: 10,
            z_config: ClusterZConfig {
                first_slice_depth: 5.0,
                far_z_mode: ClusterFarZMode::CameraFarPlane,
            },
        },
        ClusterConfig::FixedZ {
            total: 1024,
            z_slices: 10,
            z_config: ClusterZConfig {
                first_slice_depth: 5.0,
                far_z_mode: ClusterFarZMode::MaxLightRange,
            },
        },
    ];

    if key_input.just_pressed(KeyCode::C) {
        *current = (*current + 1) % configs.len();
        *q.single_mut() = configs[*current];
        println!("config: {:?}", configs[*current]);
    }
}

#[allow(clippy::too_many_arguments)]
fn camera_controller(
    mut commands: Commands,
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    key_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
    mut quit: EventWriter<AppExit>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

    if key_input.just_pressed(KeyCode::Escape) {
        quit.send(AppExit);
    }

    if let Ok((transform, _)) = query.get_single() {
        if key_input.just_pressed(KeyCode::L) {
            // red point light
            commands
                .spawn_bundle(PointLightBundle {
                    transform: Transform::from_translation(
                        transform.translation + transform.rotation * Vec3::Z * -3.0,
                    ),
                    point_light: PointLight {
                        intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
                        color: Color::RED,
                        shadows_enabled: false,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|builder| {
                    builder.spawn_bundle(PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::UVSphere {
                            radius: 0.1,
                            ..Default::default()
                        })),
                        material: materials.add(StandardMaterial {
                            base_color: Color::RED,
                            emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                            ..Default::default()
                        }),
                        ..Default::default()
                    });
                });
        }
    }
}

fn setup_cluster_diagnostics(mut diagnostics: ResMut<Diagnostics>) {
    diagnostics.add(Diagnostic::new(CLUSTER_COUNT_X, "cluster x", 1));
    diagnostics.add(Diagnostic::new(CLUSTER_COUNT_Y, "cluster y", 1));
    diagnostics.add(Diagnostic::new(CLUSTER_INDEX_COUNT, "index act", 1));
    diagnostics.add(Diagnostic::new(CLUSTER_INDEX_ESTIMATE, "index est", 1));
}

pub const CLUSTER_COUNT_X: DiagnosticId =
    DiagnosticId::from_u128(54021991829115352165418785002088010277);
pub const CLUSTER_COUNT_Y: DiagnosticId =
    DiagnosticId::from_u128(54021991829115112165418785002088010277);
pub const CLUSTER_INDEX_COUNT: DiagnosticId =
    DiagnosticId::from_u128(54021991829115352265418785002088010277);
pub const CLUSTER_INDEX_ESTIMATE: DiagnosticId =
    DiagnosticId::from_u128(54024991829115352265418785002088010277);
fn cluster_diagnostics(
    mut diagnostics: ResMut<Diagnostics>,
    clusters: Query<(&Clusters, &VisiblePointLights)>,
) {
    if let Ok((clusters, vpl)) = clusters.get_single() {
        diagnostics.add_measurement(CLUSTER_COUNT_X, clusters.axis_slices.x as f64);
        diagnostics.add_measurement(CLUSTER_COUNT_Y, clusters.axis_slices.y as f64);
        diagnostics.add_measurement(CLUSTER_INDEX_COUNT, vpl.index_count as f64);
        diagnostics.add_measurement(CLUSTER_INDEX_ESTIMATE, vpl.index_estimate as f64);
    }
}
