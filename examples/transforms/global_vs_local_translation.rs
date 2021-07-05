use bevy::prelude::*;

struct SomeEntity {
    spawn: Vec3,
    stopped: bool,
}
struct GlobalStop;
struct LocalStop;

const MAX_DISTANCE: f32 = 5.0;

impl SomeEntity {
    fn new(spawn: Vec3) -> Self {
        SomeEntity {
            spawn,
            stopped: false,
        }
    }
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(move_cubes.system())
        .add_system(stop_too_far_local_distance.system())
        .add_system(stop_too_far_global_distance.system())
        .run();
}

fn stop_too_far_global_distance(
    mut cubes: Query<(&GlobalTransform, &mut SomeEntity, &GlobalStop)>,
) {
    for (transform, mut cube, _) in cubes.iter_mut() {
        if (transform.translation - cube.spawn).length() >= MAX_DISTANCE {
            cube.stopped = true;
        }
    }
}

fn stop_too_far_local_distance(mut cubes: Query<(&Transform, &mut SomeEntity, &LocalStop)>) {
    // when checking the distance between a point with local transform as reference
    // we'll get a different outcome due to the fact that this point is in local coordinates
    // thus stopping the green cube only after reaching a distance of MAX_DISTANCE
    // to its spawn point above the now moved yellow parent block
    for (transform, mut cube, _) in cubes.iter_mut() {
        if (transform.translation - cube.spawn).length() >= MAX_DISTANCE {
            cube.stopped = true;
        }
    }
}

fn move_cubes(mut cubes: Query<(&mut Transform, &SomeEntity)>, timer: Res<Time>) {
    for (mut transform, _) in cubes.iter_mut().filter(|(_, cube)| !cube.stopped) {
        // using the local transform we can move an entity along it's local z axis
        let dir = transform.local_x();
        transform.translation += dir * timer.delta_seconds();
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::ONE * 3.0),
        ..Default::default()
    });

    let main_entity_spawn: Transform = Transform::from_translation(Vec3::ZERO);
    let subcomponent_transform = Transform::from_translation(Vec3::Y);
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(StandardMaterial {
                base_color: Color::YELLOW,
                ..Default::default()
            }),
            transform: main_entity_spawn,
            ..Default::default()
        })
        .insert(GlobalStop)
        .insert(SomeEntity::new(main_entity_spawn.translation))
        .with_children(|child_builder| {
            child_builder
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                    material: materials.add(StandardMaterial {
                        base_color: Color::RED,
                        ..Default::default()
                    }),
                    transform: subcomponent_transform,
                    ..Default::default()
                })
                .insert(GlobalStop)
                .insert(SomeEntity::new(subcomponent_transform.translation));
            child_builder
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                    material: materials.add(StandardMaterial {
                        base_color: Color::GREEN,
                        ..Default::default()
                    }),
                    transform: subcomponent_transform,
                    ..Default::default()
                })
                .insert(LocalStop)
                .insert(SomeEntity::new(subcomponent_transform.translation));
        });
}
