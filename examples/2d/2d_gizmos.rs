//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, system)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn system(mut gizmos: Gizmos, time: Res<Time>) {
    let sin = time.elapsed_seconds().sin() * 50.;
    gizmos.line_2d(Vec2::Y * -sin, Vec2::splat(-80.), Color::RED);
    gizmos.ray_2d(Vec2::Y * sin, Vec2::splat(80.), Color::GREEN);

    // Triangle
    gizmos.linestrip_gradient_2d([
        (Vec2::Y * 300., Color::BLUE),
        (Vec2::new(-255., -155.), Color::RED),
        (Vec2::new(255., -155.), Color::GREEN),
        (Vec2::Y * 300., Color::BLUE),
    ]);

    gizmos.rect_2d(
        Vec2::ZERO,
        time.elapsed_seconds() / 3.,
        Vec2::splat(300.),
        Color::BLACK,
    );

    // The circles have 32 line-segments by default.
    gizmos.circle_2d(Vec2::ZERO, 120., Color::BLACK);
    // You may want to increase this for larger circles.
    gizmos.circle_2d(Vec2::ZERO, 300., Color::NAVY).segments(64);
}
