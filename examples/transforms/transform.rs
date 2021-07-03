use bevy::prelude::*;

use std::f32::consts::PI;

struct CubeState {
    start_pos: Vec3,
    move_speed: f32,
    turn_speed: f32,
}

struct Center {
    min_size: f32,
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(move_cube.system())
        .add_system(rotate_cube.system())
        .add_system(scale_sphere_proportional_to_cube_travel_distance.system())
        .run();
}

fn move_cube(mut cubes: Query<(&mut Transform, &mut CubeState)>, timer: Res<Time>) {
    for (mut transform, cube) in cubes.iter_mut() {
        let forward = transform.forward();
        transform.translation += forward * cube.move_speed * timer.delta_seconds();
    }
}

fn rotate_cube(mut cubes: Query<(&mut Transform, &mut CubeState)>, timer: Res<Time>) {
    for (mut transform, cube) in cubes.iter_mut() {
        let full_turn = transform
            .looking_at(Vec3::ZERO, transform.local_y())
            .rotation;
        let incremental_turn_weight = cube.turn_speed * timer.delta_seconds();
        transform.rotation = transform.rotation.lerp(full_turn, incremental_turn_weight);
    }
}

fn scale_sphere_proportional_to_cube_travel_distance(
    mut transformabels: QuerySet<(
        Query<(&Transform, &CubeState)>,
        Query<(&mut Transform, &Center)>,
    )>,
) {
    let distance: f32 = transformabels
        .q0()
        .iter()
        .map(|(transform, cube)| (cube.start_pos - transform.translation).length())
        .fold(0.0, |x, y| x + y);
    for (mut transform, center) in transformabels.q1_mut().iter_mut() {
        let new_size = Vec3::ONE - 0.05 * Vec3::ONE * distance;
        let min_size = center.min_size * Vec3::ONE;
        transform.scale = if new_size.length() < min_size.length() {
            min_size
        } else {
            new_size.abs()
        }
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

    // add an object to circle around
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius: 3.0,
                subdivisions: 32,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::YELLOW,
                ..Default::default()
            }),
            transform: Transform::from_translation(Vec3::ZERO),
            ..Default::default()
        })
        .insert(Center { min_size: 0.1 });

    // add circling cube
    let mut cube_spawn = Transform::from_translation(Vec3::Z * -10.0);
    cube_spawn.rotation = Quat::from_rotation_y(PI / 2.0);
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                ..Default::default()
            }),
            transform: cube_spawn,
            ..Default::default()
        })
        .insert(CubeState {
            start_pos: cube_spawn.translation,
            move_speed: 2.0,
            turn_speed: 0.2,
        });
}
