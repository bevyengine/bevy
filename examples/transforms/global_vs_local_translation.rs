use bevy::prelude::*;

// Define a struct to store basic state information.
struct EntityState {
    spawn_location: Vec3,
    stopped: bool,
}

// Define a marker for entities that should stop at a distance in respect to their global transform.
struct GlobalStop;
// Define a marker for entities that should stop at a distance in respect to their local transform.
struct LocalStop;

// Define the maximum distance an entity should be able to move away from its spawn.
const MAX_DISTANCE: f32 = 5.0;

// Ease up creation of EntityState structs.
impl EntityState {
    fn new(spawn_location: Vec3) -> Self {
        EntityState {
            spawn_location,
            stopped: false,
        }
    }
}

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
    // This example focuses on translation only to make a simple case for the differences.
    let main_entity_spawn: Transform = Transform::from_translation(Vec3::ZERO);

    // Spawn a basic cube to have an entity as reference.
    let mut main_entity = commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::YELLOW.into()),
        transform: main_entity_spawn,
        ..Default::default()
    });
    main_entity
        .insert(GlobalStop)
        .insert(EntityState::new(main_entity_spawn.translation));

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
            .insert(EntityState::new(children_spawn.translation));
        child_builder
            .spawn_bundle(local_behaviour_child_mesh)
            .insert(LocalStop)
            .insert(EntityState::new(children_spawn.translation));
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
fn move_cubes(mut cubes: Query<(&mut Transform, &EntityState)>, timer: Res<Time>) {
    // Iterate over every entity with an EntityState that is not stopped.
    for (mut transform, _) in cubes.iter_mut().filter(|(_, cube)| !cube.stopped) {
        let dir = Vec3::X;
        transform.translation += dir * timer.delta_seconds();
    }
}

// This system will check all entities with an EntityState and the GlobalStop marker
// and check if their distance to their original spawn in respect to their global transform
// is greater than MAX_DISTANCE. If so, we mark that entity as stopped via its EntityState.
fn stop_too_far_global_distance(
    mut cubes: Query<(&GlobalTransform, &mut EntityState), With<GlobalStop>>,
) {
    for (global_transform, mut cube) in cubes.iter_mut() {
        if (global_transform.translation - cube.spawn_location).length() >= MAX_DISTANCE {
            cube.stopped = true;
        }
    }
}

// This system will do essentially the same thing as in stop_too_far_global_distance but
// using the local transform. Thus we'll check the traveling distance with respect to the
// entities parent (in the case of the green cube this would be the yellow cube).
// Since the parent (yellow) cube is also moving the green cube with LocalStop will travel further
// than its red sibling that uses the behaviour tied to GlobalStop.
fn stop_too_far_local_distance(mut cubes: Query<(&Transform, &mut EntityState), With<LocalStop>>) {
    for (transform, mut cube) in cubes.iter_mut() {
        if (transform.translation - cube.spawn_location).length() >= MAX_DISTANCE {
            cube.stopped = true;
        }
    }
}
