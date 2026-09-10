//! Example demonstrating how to use text gizmos with anchors.
//!
//! The anchor selects which part of the text is aligned to the isometry’s position:
//! `(0, 0)` center, `(-0.5, 0.0)` left edge, `(0.0, 0.5)` top edge.

use bevy::color::palettes::css::{BLUE, GREEN, ORANGE, RED, YELLOW};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_camera)
        .add_systems(Update, anchors)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn anchors(mut text_gizmos: Gizmos, time: Res<Time>) {
    let t = time.elapsed_secs();
    for (label, anchor, color) in [
        ("left", vec2(-0.5, 0.0), RED),
        ("right", vec2(0.5, 0.0), ORANGE),
        ("center", Vec2::ZERO, YELLOW),
        ("top", vec2(0.0, 0.5), GREEN),
        ("bottom", vec2(0.0, -0.5), BLUE),
    ] {
        let position = Vec2::splat(350.0) * anchor;
        text_gizmos.text_2d(
            Isometry2d::from_translation(position),
            "+",
            12.,
            Vec2::ZERO,
            Color::WHITE,
        );
        text_gizmos.text_2d(
            Isometry2d::new(position, Rot2::radians(t)),
            label,
            25.,
            anchor,
            color,
        );
    }
}
