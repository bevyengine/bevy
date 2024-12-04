//! This example demonstrates Bevy's immediate mode animated drawing API intended for visual debugging.

use bevy::{color::palettes::css::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, draw_example_collection)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 7.0).looking_at(Vec3::ZERO, Vec3::Z),
        ..Default::default()
    });
}

fn draw_example_collection(mut gizmos: AnimatedGizmos) {
    let colors = [
        RED, ORANGE, YELLOW, DARK_GREEN, GREEN, LIGHT_BLUE, LIGHT_CYAN, AZURE, BLUE, VIOLET,
    ];
    (1..=10).zip(colors).for_each(|(n, color)| {
        let speed = n as f32 * 0.1;
        let offset = Vec3::Y * n as f32 * 0.1;
        gizmos
            .animated_line(
                Vec3::X + offset - Vec3::ONE * 0.5 - Vec3::Y * 0.5,
                Vec3::Y + offset - Vec3::ONE * 0.5 - Vec3::Y * 0.5,
                color,
            )
            .segments(n)
            .speed(speed);
    });
}
