//! Useful for stress-testing rendering cases that exhibit extreme color banding.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::BLACK,
            brightness: 0.0,
        })
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let colors = [Color::WHITE, Color::RED, Color::GREEN, Color::BLUE];
    let x = [-5.0, 5.0, -5.0, 5.0];
    let z = [-5.0, -5.0, 5.0, 5.0];
    let mesh = meshes.add(Mesh::from(shape::Plane { size: 10.0 }));

    for i in 0..4 {
        // plane
        commands.spawn_bundle(PbrBundle {
            mesh: mesh.clone(),
            material: materials.add(custom_material(colors[i])),
            transform: Transform::from_xyz(x[i], 0.0, z[i]),
            ..default()
        });
    }
    // light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 50.0,
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 5.0, 0.0),
        ..default()
    });
    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 6.0, 0.0).looking_at(Vec3::default(), -Vec3::Z),
        ..default()
    });
}

fn custom_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.0,
        ..default()
    }
}
