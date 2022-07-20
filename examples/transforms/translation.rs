//! Illustrates how to move an object along an axis.

use bevy::prelude::*;

// Define a struct to keep some information about our entity.
// Here it's an arbitrary movement speed, the spawn location, and a maximum distance from it.
#[derive(Component)]
struct Movable {
    spawn: Vec3,
    max_distance: f32,
    speed: f32,
}

// Implement a utility function for easier Movable struct creation.
impl Movable {
    fn new(spawn: Vec3) -> Self {
        Movable {
            spawn,
            max_distance: 5.0,
            speed: 2.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(move_cube)
        .run();
}

// Startup system to setup the scene and spawn all relevant entities.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Add a cube to visualize translation.
    let entity_spawn = Vec3::ZERO;
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::WHITE.into()),
            transform: Transform::from_translation(entity_spawn),
            ..default()
        })
        .insert(Movable::new(entity_spawn));

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 10.0, 20.0).looking_at(entity_spawn, Vec3::Y),
        ..default()
    });

    // Add a light source for better 3d visibility.
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::ONE * 3.0),
        ..default()
    });
}

// This system will move all Movable entities with a Transform
fn move_cube(mut cubes: Query<(&mut Transform, &mut Movable)>, timer: Res<Time>) {
    for (mut transform, mut cube) in &mut cubes {
        // Check if the entity moved too far from its spawn, if so invert the moving direction.
        if (cube.spawn - transform.translation).length() > cube.max_distance {
            cube.speed *= -1.0;
        }
        let direction = transform.local_x();
        transform.translation += direction * cube.speed * timer.delta_seconds();
    }
}
