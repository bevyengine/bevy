use bevy::prelude::*;

struct Scaling {
    scale_axis: Vec3,
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(scale_cube.system())
        .run();
}

fn scale_cube(mut cubes: Query<(&mut Transform, &mut Scaling)>, timer: Res<Time>) {
    for (mut transform, mut cube) in cubes.iter_mut() {
        if transform.scale.length() >= 10.0 {
            cube.scale_axis *= -1.0;
        } else if transform.scale.length() < Vec3::ONE.length() {
            // switch to next axis and expand
            transform.scale = Vec3::ONE;
            cube.scale_axis *= -1.0;
            cube.scale_axis = Vec3::from((cube.scale_axis.z, cube.scale_axis.x, cube.scale_axis.y));
        }
        transform.scale += cube.scale_axis * timer.delta_seconds();
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
        .insert(Scaling {
            scale_axis: Vec3::X * 2.0,
        });
}
