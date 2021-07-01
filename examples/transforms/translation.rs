use bevy::prelude::*;

struct Movable {
    speed: f32,
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(move_cube.system())
        .run();
}

fn move_cube(mut cubes: Query<(&mut Transform, &mut Movable)>, timer: Res<Time>) {
    for (mut transform, mut cube) in cubes.iter_mut() {
        if transform.translation.length() > 10.0 {
            cube.speed *= -1.0;
        }
        transform.translation += Vec3::X * cube.speed * timer.delta_seconds();
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
        .insert(Movable { speed: 2.0 });
}
