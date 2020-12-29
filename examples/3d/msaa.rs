use bevy::prelude::*;
//use bevy_pbr::{LightBundle, PbrBundle};

/// This example shows how to configure Multi-Sample Anti-Aliasing. Setting the sample count higher will result in smoother edges,
/// but it will also increase the cost to render those edges. The range should generally be somewhere between 1 (no multi sampling,
/// but cheap) to 8 (crisp but expensive)
fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .run();
}

/// set up a simple 3D scene
fn setup(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add entities to the world
    commands
        // cube
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius: 1.0,
                subdivisions: 32,
            })),
            material: materials.add(Color::rgb(0.5, 0.4, 0.3).into()),
            ..Default::default()
        })
        // light
        .spawn(LightBundle {
            transform: Transform::from_translation(Vec3::new(40.0, 80.0, 40.0)),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(-3.0, 3.0, 5.0))
                .looking_at(Vec3::default(), Vec3::unit_y()),
            ..Default::default()
        });
}
