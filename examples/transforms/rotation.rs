use bevy::prelude::*;

use std::f32::consts::PI;

const FULL_TURN: f32 = 2.0 * PI;

struct Rotatable {
    speed: f32,
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(rotate_cube.system())
        .run();
}

fn rotate_cube(mut cubes: Query<(&mut Transform, &Rotatable)>, timer: Res<Time>) {
    for (mut transform, cube) in cubes.iter_mut() {
        let rotation_change = Quat::from_rotation_y(FULL_TURN * cube.speed * timer.delta_seconds());
        transform.rotate(rotation_change);
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

    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                ..Default::default()
            }),
            transform: Transform::from_translation(Vec3::ZERO),
            ..Default::default()
        })
        .insert(Rotatable { speed: 0.3 });
}
