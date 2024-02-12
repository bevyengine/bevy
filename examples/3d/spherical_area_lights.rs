//! Demonstrates how lighting is affected by different radius of point lights.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.2, 1.5, 2.5).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(100.0, 100.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.2, 0.2, 0.2),
            perceptual_roughness: 0.08,
            ..default()
        }),
        ..default()
    });

    const COUNT: usize = 6;
    let position_range = -2.0..2.0;
    let radius_range = 0.0..0.4;
    let pos_len = position_range.end - position_range.start;
    let radius_len = radius_range.end - radius_range.start;
    let mesh = meshes.add(Sphere::new(0.5).mesh().uv(120, 64));

    for i in 0..COUNT {
        let percent = i as f32 / COUNT as f32;
        let radius = radius_range.start + percent * radius_len;

        // sphere light
        commands
            .spawn(PbrBundle {
                mesh: mesh.clone(),
                material: materials.add(StandardMaterial {
                    base_color: Color::rgb(0.5, 0.5, 1.0),
                    unlit: true,
                    ..default()
                }),
                transform: Transform::from_xyz(position_range.start + percent * pos_len, 0.3, 0.0)
                    .with_scale(Vec3::splat(radius)),
                ..default()
            })
            .with_children(|children| {
                children.spawn(PointLightBundle {
                    point_light: PointLight {
                        intensity: 100_000.0,
                        radius,
                        color: Color::rgb(0.2, 0.2, 1.0),
                        ..default()
                    },
                    ..default()
                });
            });
    }
}
