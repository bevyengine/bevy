//! Shows that multiple instances of a cube are automatically instanced in one draw call
//! Try running this example in a graphics profiler and all the cubes should be only a single draw call.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 48.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 16.0, 8.0),
    ));

    let mesh = meshes.add(Cuboid::from_size(Vec3::splat(0.5)));
    // This example uses the StandardMaterial but it can work with most custom material too
    let material = materials.add(Color::srgb_u8(124, 144, 255));

    // spawn 1000 cubes
    for x in -5..5 {
        for y in -5..5 {
            for z in -5..5 {
                commands.spawn((
                    // For automatic instancing to take effect you need to
                    // use the same mesh handle and material handle for each instance
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(x as f32, y as f32, z as f32),
                ));
            }
        }
    }
}
