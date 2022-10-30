//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(system)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn system(mut draw: ResMut<DebugDraw>, time: Res<Time>) {
    draw.line_2d(Vec2::ZERO, Vec2::new(-200., 300.), Color::RED);
    draw.line_2d(Vec2::ZERO, Vec2::ONE * 300., Color::GREEN);

    draw.rect_2d(
        Vec2::ZERO,
        time.elapsed_seconds(),
        Vec2::ONE * 300.,
        Color::BLACK,
    );
    // The circles have 24 line-segments by default.
    draw.circle_2d(Vec2::ZERO, 120., Color::BLACK);
    // You may want to increase this for larger circles.
    draw.circle_segments = 64;
    draw.circle_2d(Vec2::ZERO, 250., Color::NAVY);
    draw.circle_segments = 24;
}
