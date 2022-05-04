use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(light_movement)
        .add_system(movement)
        .add_system(debug)
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
        mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0 })),
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
        color: Color::ORANGE_RED,
        brightness: 0.02,
    });

    // red spotlight
    commands
        .spawn_bundle(PointLightBundle {
            transform: Transform::from_xyz(1.0, 2.0, 0.0).looking_at(-Vec3::Y, Vec3::Z),
            point_light: PointLight {
                intensity: 200.0, // lumens - roughly a 100W non-halogen incandescent bulb
                color: Color::RED,
                shadows_enabled: true,
                spotlight_angles: Some((std::f32::consts::PI / 4.0 * 0.8, std::f32::consts::PI / 4.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Torus {
                    radius: 0.1,
                    ring_radius: 0.05,
                    ..default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::RED,
                    emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                    ..default()
                }),
                ..default()
            });
            builder.spawn_bundle(PbrBundle {
                transform: Transform::from_translation(Vec3::Z * -0.5),
                mesh: meshes.add(Mesh::from(shape::Torus {
                    radius: 0.1,
                    ring_radius: 0.05,
                    ..default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::RED,
                    emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                    ..default()
                }),
                ..default()
            }).insert(DebugMe);
        });

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn light_movement(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<PointLight>>,
) {
    for mut transform in query.iter_mut() {
        transform.look_at(-Vec3::Y + Vec3::X - Vec3::X * 1.33 * (1.5 * time.seconds_since_startup()).sin() as f32, Vec3::Z);
    }
}

#[derive(Component)]
struct DebugMe;

fn debug(
    q: Query<(Entity, &Transform, &GlobalTransform), With<DebugMe>>,
    time: Res<Time>,
    mut local: Local<usize>,
) {
    let secs = time.seconds_since_startup() as usize;
    if secs > *local {
        println!("---");
        for (e, t, g) in q.iter() {
            println!("e: {:?}, t: {:?}, g: {:?}", e, t.translation, g.translation);
        }
        *local = secs;
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
