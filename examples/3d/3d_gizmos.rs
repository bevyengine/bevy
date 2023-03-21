//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.
use std::f32::consts::PI;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (system, rotate_camera, update_config))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0., 1.5, 6.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane::from_size(5.0))),
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
    // text
    commands.spawn(TextBundle::from_section(
        "Press 't' to toggle drawing gizmos on top of everything else in the scene",
        TextStyle {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            font_size: 24.,
            color: Color::WHITE,
        },
    ));
}

fn system(mut gizmos: Gizmos, time: Res<Time>) {
    gizmos.cuboid(
        Vec3::Y * -0.5,
        Quat::IDENTITY,
        Vec3::new(5., 1., 2.),
        Color::BLACK,
    );
    gizmos.rect(
        Vec3::new(time.elapsed_seconds().cos() * 2.5, 1., 0.),
        Quat::from_rotation_y(PI / 2.),
        Vec2::splat(2.),
        Color::GREEN,
    );

    gizmos.sphere(
        Vec3::new(1., 0.5, 0.),
        Quat::IDENTITY,
        0.5,
        Color::RED.with_a(0.5),
    );

    for y in [0., 0.5, 1.] {
        gizmos.ray(
            Vec3::new(1., y, 0.),
            Vec3::new(-3., (time.elapsed_seconds() * 3.).sin(), 0.),
            Color::BLUE,
        );
    }

    // Circles have 32 line-segments by default.
    gizmos.circle(Vec3::ZERO, Vec3::Y, 3., Color::BLACK);
    // You may want to increase this for larger circles or spheres.
    gizmos
        .circle(Vec3::ZERO, Vec3::Y, 3.1, Color::NAVY)
        .segments(64);
    gizmos
        .sphere(Vec3::ZERO, Quat::IDENTITY, 3.2, Color::BLACK)
        .circle_segments(64);
}

fn rotate_camera(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    let mut transform = query.single_mut();

    transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_seconds() / 2.));
}

fn update_config(mut gizmo_config: ResMut<GizmoConfig>, keyboard: Res<Input<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::T) {
        gizmo_config.on_top = !gizmo_config.on_top;
    }
}
