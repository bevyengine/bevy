//! Illustrates the use of vertex colors.

use bevy::{prelude::*, render::mesh::VertexAttributeValues};

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
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(5.0, 5.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });
    // cube
    // Assign vertex colors based on vertex positions
    let mut colorful_cube = Mesh::from(Cuboid::default());
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        colorful_cube.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        let colors: Vec<[f32; 4]> = positions
            .iter()
            .map(|[r, g, b]| [(1. - *r) / 2., (1. - *g) / 2., (1. - *b) / 2., 1.])
            .collect();
        colorful_cube.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    commands.spawn(PbrBundle {
        mesh: meshes.add(colorful_cube),
        // This is the default color, but note that vertex colors are
        // multiplied by the base color, so you'll likely want this to be
        // white if using vertex colors.
        material: materials.add(Color::srgb(1., 1., 1.)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 5.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
