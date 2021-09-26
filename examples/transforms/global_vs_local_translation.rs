use bevy::prelude::*;

// Define structs to store state information.
struct Moving;
struct SpawnLocation(Vec3);

// Define a marker for entities that should stop at a distance in respect to their global transform.
struct GlobalStop;
// Define a marker for entities that should stop at a distance in respect to their local transform.
struct LocalStop;

// Define the maximum distance an entity should be able to move away from its spawn.
const MAX_DISTANCE: f32 = 5.0;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(move_cubes.system())
        .add_system(stop_too_far_global_distance.system())
        .add_system(stop_too_far_local_distance.system())
        .run();
}

// Startup system to setup the scene and spawn all relevant entities.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // To show the difference between a local transform (rotation, scale and position in respect to a given entity)
    // and global transform (rotation, scale and position in respect to the base coordinate system of the visible scene)
    // it's helpful to add multiple entities that are attached to each other.
    // This way we'll see that the transform in respect to an entities parent is different to the
    // global transform within the visible scene.
    // This example focuses on translation only to clearly demonstrate the differences.
    let main_entity_spawn: Transform = Transform::from_translation(Vec3::ZERO);

    // Spawn a basic cube to have an entity as reference.
    let mut main_entity = commands.spawn();
    main_entity
        .insert_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::YELLOW.into()),
            transform: main_entity_spawn,
            ..Default::default()
        })
        .insert(GlobalStop)
        .insert(Moving)
        .insert(SpawnLocation(main_entity_spawn.translation));

    // Define a spawn point for child entities just above the original entity.
    let children_spawn = Transform::from_translation(Vec3::Y);
    let global_behaviour_child_mesh = PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
        material: materials.add(Color::RED.into()),
        transform: children_spawn,
        ..Default::default()
    };
    let local_behaviour_child_mesh = PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
        material: materials.add(Color::GREEN.into()),
        transform: children_spawn,
        ..Default::default()
    };

    // Spawn two entities as children above the original main entity.
    // The red entity spawned here will change its behaviour depending on its global transform
    // where the green one will change its behaviour depending on its local transform.
    main_entity.with_children(|child_builder| {
        // also see parenting example
        child_builder
            .spawn_bundle(global_behaviour_child_mesh)
            .insert(GlobalStop)
            .insert(Moving)
            .insert(SpawnLocation(children_spawn.translation));
        child_builder
            .spawn_bundle(local_behaviour_child_mesh)
            .insert(LocalStop)
            .insert(Moving)
            .insert(SpawnLocation(children_spawn.translation));
    });

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 10.0, 20.0)
            .looking_at(main_entity_spawn.translation, Vec3::Y),
        ..Default::default()
    });

    // Add a light source for better 3d visibility.
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::ONE * 3.0),
        ..Default::default()
    });
}

// This system will move all entities that have an EntityState (all cubes).
// The movement here is restricted to one direction (right in the sense of the cameras view).
fn move_cubes(mut cubes: Query<&mut Transform, With<Moving>>, timer: Res<Time>) {
    // Iterate over every entity with an EntityState that is not stopped.
    for mut transform in cubes.iter_mut() {
        let dir = Vec3::X;
        transform.translation += dir * timer.delta_seconds();
    }
}

// This system will check all entities with an EntityState and the GlobalStop marker
// and check if their distance to their original spawn in respect to their global transform
// is greater than MAX_DISTANCE. If so, we mark that entity as stopped via its EntityState.
fn stop_too_far_global_distance(
    mut commands: Commands,
    mut cubes: Query<(Entity, &GlobalTransform, &SpawnLocation), With<GlobalStop>>,
) {
    for (entity, global_transform, spawn) in cubes.iter_mut() {
        if (global_transform.translation - spawn.0).length() >= MAX_DISTANCE {
            commands.entity(entity).remove::<Moving>();
        }
    }
}

// This system will do essentially the same thing as in stop_too_far_global_distance but
// using the local transform. Thus we'll check the traveling distance with respect to the
// entities parent (in the case of the green cube this would be the yellow cube).
// Since the parent (yellow) cube is also moving the green cube with LocalStop will travel further
// than its red sibling that uses the behaviour tied to GlobalStop.
fn stop_too_far_local_distance(
    mut commands: Commands,
    mut cubes: Query<(Entity, &Transform, &SpawnLocation), With<LocalStop>>,
) {
    for (entity, transform, spawn) in cubes.iter_mut() {
        if (transform.translation - spawn.0).length() >= MAX_DISTANCE {
            commands.entity(entity).remove::<Moving>();
        }
    }
}
