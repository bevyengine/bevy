use bevy::prelude::*;

/// This example shows the difference between flat shading
/// and smooth shading (default) in `StandardMaterial`.
/// Flat shading gives a much more "Polygonal" or "Retro" look to meshes.
fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Flat shaded icosphere (ORANGE)
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            radius: 0.5,
            subdivisions: 4,
        })),
        material: materials.add(StandardMaterial {
            base_color: Color::ORANGE,
            flat_shading: true,
            ..Default::default()
        }),
        transform: Transform::from_xyz(-0.55, 0.5, 0.0),
        ..Default::default()
    });
    // Smooth shaded icosphere (BLUE)
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            radius: 0.5,
            subdivisions: 4,
        })),
        material: materials.add(Color::BLUE.into()),
        transform: Transform::from_xyz(0.55, 0.5, 0.0),
        ..Default::default()
    });

    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(1.0, 3.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}
