//! A simple 3D scene with light shining over a cube sitting on a plane.

use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        //.add_systems(Update, update)
        .run();
}

#[derive(Component)]
struct Marker;

fn update(time: Res<Time>, mut commands: Commands, delete: Query<Entity, With<Marker>>) {
    if let Some(delete) = delete.get_single().ok() {
        if time.elapsed_seconds() > 5.0 {
            commands.entity(delete).despawn();
        }
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Circle::new(4.0)),
            material: materials.add(Color::WHITE),
            transform: Transform::from_rotation(Quat::from_rotation_x(
                -std::f32::consts::FRAC_PI_2,
            )),
            ..default()
        },
        NotShadowCaster,
    ));
    // cube
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube { size: 1.0 }),
            material: materials.add(Color::rgb_u8(124, 144, 255)),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        Marker,
    ));
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 250_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
