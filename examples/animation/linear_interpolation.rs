//! Demonstrates how linear interpolation can be used with bevy types.

use std::f32::consts::PI;

use bevy::prelude::*;

#[derive(Component)]
pub enum LerpExample {
    Translation(Vec3, Vec3),
    Rotation(Quat, Quat),
    Scale(f32, f32),
    Color(Color, Color),
}

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

    // Spawning cubes to experiment on
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube::default().into()),
            material: materials.add(Color::ORANGE.into()),
            transform: Transform::from_xyz(-6., 2., 0.)
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            ..default()
        },
        LerpExample::Translation(Vec3::new(-6., 2., 0.), Vec3::new(-6., 4., 0.)),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube::default().into()),
            material: materials.add(Color::ORANGE.into()),
            transform: Transform::from_xyz(-2., 2., 0.)
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            ..default()
        },
        LerpExample::Rotation(Quat::from_rotation_x(-PI / 4.), Quat::from_rotation_z(PI)),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube::default().into()),
            material: materials.add(Color::ORANGE.into()),
            transform: Transform::from_xyz(2., 2., 0.)
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            ..default()
        },
        LerpExample::Scale(1., 2.),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube::default().into()),
            material: materials.add(Color::ORANGE.into()),
            transform: Transform::from_xyz(6., 2., 0.)
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            ..default()
        },
        LerpExample::Color(Color::ORANGE, Color::PURPLE),
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

pub fn linear_interpolate(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Handle<StandardMaterial>, &LerpExample)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // s lies between 0 and 1
    let s = (time.elapsed_seconds().sin() + 1.) / 2.;

    for (mut transform, handle, lerp_example) in &mut query {
        match lerp_example {
            LerpExample::Translation(origin, target) => {
                // Linear interpolation on Vec (Vec2 or Vec3, they are used the same way)
                transform.translation = origin.lerp(*target, s);
            }
            LerpExample::Rotation(origin, target) => {
                // Spherical linear interpolation on Quat
                transform.rotation = origin.slerp(*target, s);
            }
            LerpExample::Scale(origin, target) => {
                // Lerp on f32 directly
                transform.scale = Vec3::splat(origin.lerp(*target, s))
            }
            LerpExample::Color(origin, target) => {
                let Some(mut mat) = materials.get_mut(&handle) else { continue; };

                // Lerp on colors
                mat.base_color = origin.lerp(*target, s);
            }
        }
    }
}
