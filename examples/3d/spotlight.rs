use bevy::{pbr::NotShadowCaster, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(light_sway)
        .add_system(movement)
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
        mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
        material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            ..default()
        }),
        ..default()
    });

    // cube
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(StandardMaterial {
                base_color: Color::PINK,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        })
        .insert(Movable);

    // ambient light
    commands.insert_resource(AmbientLight {
        color: Color::BLUE,
        brightness: 0.04,
    });

    // red spotlight
    commands
        .spawn_bundle(PointLightBundle {
            transform: Transform::from_xyz(1.0, 2.0, 0.0),
            point_light: PointLight {
                intensity: 200.0, // lumens
                color: Color::WHITE,
                shadows_enabled: true,
                spotlight_angles: Some((
                    std::f32::consts::PI / 4.0 * 0.85,
                    std::f32::consts::PI / 4.0,
                )),
                ..default()
            },
            ..default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.05,
                    ..default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::RED,
                    emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                    ..default()
                }),
                ..default()
            });
            builder
                .spawn_bundle(PbrBundle {
                    transform: Transform::from_translation(Vec3::Z * -0.1),
                    mesh: meshes.add(Mesh::from(shape::UVSphere {
                        radius: 0.1,
                        ..default()
                    })),
                    material: materials.add(StandardMaterial {
                        base_color: Color::RED,
                        emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                        ..default()
                    }),
                    ..default()
                })
                .insert(NotShadowCaster);
        });

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-4.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn light_sway(time: Res<Time>, mut query: Query<(&mut Transform, &mut PointLight)>) {
    for (mut transform, mut light) in query.iter_mut() {
        transform.rotation = Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_2,
            time.seconds_since_startup().sin() as f32 * 0.75,
            0.0,
        );
        let angle =
            ((time.seconds_since_startup() * 1.2).sin() as f32 + 1.0) * std::f32::consts::FRAC_PI_4;
        light.spotlight_angles = Some((angle * 0.8, angle));
    }
}

fn movement(
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Movable>>,
) {
    for mut transform in query.iter_mut() {
        let mut direction = Vec3::ZERO;
        if input.pressed(KeyCode::Up) {
            direction.y += 1.0;
        }
        if input.pressed(KeyCode::Down) {
            direction.y -= 1.0;
        }
        if input.pressed(KeyCode::Left) {
            direction.x -= 1.0;
        }
        if input.pressed(KeyCode::Right) {
            direction.x += 1.0;
        }

        transform.translation += time.delta_seconds() * 2.0 * direction;
    }
}
