use bevy::{pbr::BloomSettings, prelude::*};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(bounce)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true, // 1. HDR must be enabled
                ..default()
            },
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        BloomSettings::default(), // 2. Enable bloom
    ));

    let mesh = meshes.add(
        shape::Icosphere {
            radius: 0.5,
            subdivisions: 5,
        }
        .into(),
    );

    let material = materials.add(StandardMaterial {
        emissive: Color::rgb_linear(1.0, 0.3, 0.2) * 4.0,
        ..Default::default()
    });
    let material_non_emissive = materials.add(StandardMaterial {
        ..Default::default()
    });

    for x in -10..10 {
        for z in -10..10 {
            let mut hasher = DefaultHasher::new();
            (x, z).hash(&mut hasher);
            let rand = hasher.finish() % 2 == 0;

            let material = match rand {
                true => material.clone(),
                false => material_non_emissive.clone(),
            };

            commands.spawn((
                PbrBundle {
                    mesh: mesh.clone(),
                    material,
                    transform: Transform::from_xyz(x as f32 * 2.0, 0.0, z as f32 * 2.0),
                    ..Default::default()
                },
                Bouncing,
            ));
        }
    }
}

#[derive(Component)]
struct Bouncing;

fn bounce(time: Res<Time>, mut query: Query<&mut Transform, With<Bouncing>>) {
    for mut transform in query.iter_mut() {
        transform.translation.y =
            (transform.translation.x + transform.translation.z + time.elapsed_seconds() as f32)
                .sin();
    }
}
