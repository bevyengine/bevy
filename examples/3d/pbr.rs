//! This example shows how to configure Physically Based Rendering (PBR) parameters.

use bevy::prelude::*;

fn main() {
    App::new()
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
    // add entities to the world
    for y in -2..=2 {
        for x in -5..=5 {
            let x01 = (x + 5) as f32 / 10.0;
            let y01 = (y + 2) as f32 / 4.0;
            // sphere
            commands.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Icosphere {
                    radius: 0.45,
                    subdivisions: 32,
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::hex("ffd891").unwrap(),
                    // vary key PBR parameters on a grid of spheres to show the effect
                    metallic: y01,
                    perceptual_roughness: x01,
                    ..default()
                }),
                transform: Transform::from_xyz(x as f32, y as f32 + 0.5, 0.0),
                ..default()
            });
        }
    }
    // unlit sphere
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            radius: 0.45,
            subdivisions: 32,
        })),
        material: materials.add(StandardMaterial {
            base_color: Color::hex("ffd891").unwrap(),
            // vary key PBR parameters on a grid of spheres to show the effect
            unlit: true,
            ..default()
        }),
        transform: Transform::from_xyz(-5.0, -2.5, 0.0),
        ..default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(50.0, 50.0, 50.0),
        point_light: PointLight {
            intensity: 600000.,
            range: 100.,
            ..default()
        },
        ..default()
    });
    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::default(), Vec3::Y),
        projection: OrthographicProjection {
            scale: 0.01,
            ..default()
        }
        .into(),
        ..default()
    });
}
