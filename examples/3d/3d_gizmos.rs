//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.
use std::f32::consts::PI;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(system)
        .add_system(rotate_camera)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0., 1.5, 6.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn system(mut draw: Gizmos, time: Res<Time>) {
    draw.cuboid(
        Vec3::Y * -0.5,
        Quat::IDENTITY,
        Vec3::new(5., 1., 2.),
        Color::BLACK,
    );
    draw.rect(
        Vec3::new(time.elapsed_seconds().cos() * 2.5, 1., 0.),
        Quat::from_rotation_y(PI / 2.),
        Vec2::splat(2.),
        Color::GREEN,
    );

    draw.sphere(Vec3::new(1., 0.5, 0.), 0.5, Color::RED);

    for y in [0., 0.5, 1.] {
        draw.ray(
            Vec3::new(1., y, 0.),
            Vec3::new(-3., (time.elapsed_seconds() * 3.).sin(), 0.),
            Color::BLUE,
        );
    }

    // Circles have 32 line-segments by default.
    draw.circle(Vec3::ZERO, Vec3::Y, 3., Color::BLACK);
    // You may want to increase this for larger circles or spheres.
    draw.circle(Vec3::ZERO, Vec3::Y, 3.1, Color::NAVY)
        .segments(64);
    draw.sphere(Vec3::ZERO, 3.2, Color::BLACK)
        .circle_segments(64);
}

fn rotate_camera(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    let mut transform = query.single_mut();

    transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_seconds() / 2.));
}
