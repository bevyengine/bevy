//! The Simple 3D Scene with user-defined clipping enabled.

use bevy::prelude::*;

#[derive(Component)]
struct MainCamera;
#[derive(Component)]
struct ReflectionCamera;

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
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // camera, here we add a custom clipping plane to the camera.
    // This causes the fragment shader to reject fragments below the defined plane
    // The idea of this is similar to that of the gl_Clipdistance in OpenGL.
    // User clipping is used when making reflective surface like mirrors or water, where
    // we dont want to render objects behind the mirror.
    commands.spawn((
        MainCamera,
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                user_defined_clipping_plane: Some(Vec4::new(-1.0, 0.0, 1.0, 0.0).normalize()),
                ..default()
            },
            ..default()
        },
    ));
}
