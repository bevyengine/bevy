//! A Bevy app that you can connect to with the BRP and edit.

use bevy::math::ops::cos;
use bevy::{
    input::common_conditions::input_just_pressed,
    prelude::*,
    remote::{http::RemoteHttpPlugin, RemotePlugin},
};
use serde::{Deserialize, Serialize};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, remove.run_if(input_just_pressed(KeyCode::Space)))
        .add_systems(Update, move_cube)
        .register_type::<Cube>()
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Cube(1.0),
    ));

    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn move_cube(mut query: Query<&mut Transform, With<Cube>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.translation.y = -cos(time.elapsed_secs()) + 1.5;
    }
}

fn remove(mut commands: Commands, cube_entity: Single<Entity, With<Cube>>) {
    commands.entity(*cube_entity).remove::<Cube>();
}

#[derive(Component, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
struct Cube(f32);
