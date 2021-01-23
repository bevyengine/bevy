use bevy::prelude::*;

// the `bevy_main` proc_macro generates the required android boilerplate
#[bevy_main]
fn main() {
    App::build()
        //.add_resource(Msaa { samples: 2 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(rotate.system())
        .run();
}

struct Rotation;

/// set up a simple 3D scene
fn setup(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add entities to the world
    commands
        // plane
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..Default::default()
        })
        // cube
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..Default::default()
        }).with(Rotation)

        // light
        .spawn(LightBundle {
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0)
                .looking_at(Vec3::default(), Vec3::unit_y()),
            ..Default::default()
        });
}

fn rotate(
    time: Res<Time>,
    mut transform_query: Query<&mut Transform, With<Rotation>>,
) {
    let angle = std::f32::consts::PI / 2.0;

    for mut transform in transform_query.iter_mut() {
        transform.rotate(Quat::from_rotation_y(angle * 0.5 * time.delta_seconds()));
    }

}
