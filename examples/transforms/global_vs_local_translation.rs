use bevy::prelude::*;

// Define a marker for entities that should be changed via their global transform.
struct ChangeGlobal;
// Define a marker for entities that should be changed via their local transform.
struct ChangeLocal;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(move_cubes_according_to_global_transform)
        .add_system(move_cubes_according_to_local_transform)
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
    // This way we'll see that the transform in respect to an entity's parent is different to the
    // global transform within the visible scene.
    // This example focuses on translation only to clearly demonstrate the differences.

    // Spawn a basic cube to have an entity as reference.
    let mut main_entity = commands.spawn();
    main_entity
        .insert_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::YELLOW.into()),
            transform: Transform::from_translation(Vec3::ZERO),
            ..Default::default()
        })
        .insert(ChangeLocal);

    // Spawn two entities as children above the original main entity.
    // The red entity spawned here will be changed via its global transform
    // where the green one will be changed via its local transform.
    main_entity.with_children(|child_builder| {
        // also see parenting example
        child_builder
            .spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                material: materials.add(Color::RED.into()),
                transform: Transform::from_translation(Vec3::Y - Vec3::Z),
                ..Default::default()
            })
            .insert(ChangeGlobal);
        child_builder
            .spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                material: materials.add(Color::GREEN.into()),
                transform: Transform::from_translation(Vec3::Y + Vec3::Z),
                ..Default::default()
            })
            .insert(ChangeLocal);
    });

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    // Add a light source for better 3d visibility.
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::ONE * 3.0),
        ..Default::default()
    });
}

// This system will move all cubes that are marked as ChangeGlobal according to their global transform.
fn move_cubes_according_to_global_transform(
    keyboard_input: Res<Input<KeyCode>>,
    mut cubes: Query<&mut GlobalTransform, With<ChangeGlobal>>,
    timer: Res<Time>,
) {
    for mut global_transform in cubes.iter_mut() {
        let direction = direction_from_input(&keyboard_input);
        global_transform.translation += Vec3::X * direction * timer.delta_seconds();
    }
}

// This system will move all cubes that are marked as ChangeLocal according to their local transform.
fn move_cubes_according_to_local_transform(
    keyboard_input: Res<Input<KeyCode>>,
    mut cubes: Query<&mut Transform, With<ChangeLocal>>,
    timer: Res<Time>,
) {
    for mut transform in cubes.iter_mut() {
        let direction = direction_from_input(&keyboard_input);
        transform.translation += Vec3::X * direction * timer.delta_seconds();
    }
}

// A quick helper function to determine the cubes movement direction based on left/right-input
fn direction_from_input(keyboard_input: &Res<Input<KeyCode>>) -> f32 {
    (keyboard_input.pressed(KeyCode::Right) as i32 - keyboard_input.pressed(KeyCode::Left) as i32)
        as f32
}
