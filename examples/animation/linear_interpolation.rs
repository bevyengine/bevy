//! Demonstrates how linear interpolation can be used with bevy types.

use std::f32::consts::PI;

use bevy::prelude::*;

#[derive(Component)]
pub struct Sphere;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, linear_interpolate)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawning a sphere to experiment on
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube::default().into()),
            material: materials.add(Color::ORANGE.into()),
            transform: Transform::from_xyz(0., 2., 0.),
            ..default()
        },
        Sphere,
    ));

    // Some light to see something
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.,
            range: 100.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(8., 16., 8.),
        ..default()
    });

    // ground plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(50.).into()),
        material: materials.add(Color::SILVER.into()),
        ..default()
    });

    // The camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0., 6., 12.).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..default()
    });
}

pub fn linear_interpolate(time: Res<Time>, mut query: Query<&mut Transform, With<Sphere>>) {
    // s lies between 0 and 1
    let s = (time.elapsed_seconds().sin() + 1.) / 2.;

    for mut transform in &mut query {
        // // Linear interpolation on Vec3 (lerp on Vec2 is the same format)
        transform.translation = Vec3::new(0., 2., 0.).lerp(Vec3::new(0., 4., 0.), s);

        // Spherical linear interpolation on Quat
        transform.rotation = Quat::from_rotation_x(-PI / 4.).slerp(Quat::from_rotation_z(PI), s);

        // Lerp on f32 directly
        transform.scale = Vec3::splat((1.).lerp(2., s));
    }
}
