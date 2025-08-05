//! Gizmo Playground Example
//!
//! This example demonstrates how to render various gizmos in a Bevy window.
//! Gizmos are debug visualization tools that help with development and debugging.
//!
//! Key concepts:
//! - Gizmos are rendered using the `Gizmos` resource
//! - They are drawn in 3D space and automatically handled by the renderer
//! - Gizmos are useful for visualizing transforms, bounds, paths, and more
//!
//! Controls:
//! - Use mouse to rotate the camera view
//! - Scroll to zoom in/out
//! - Right-click and drag to pan

use bevy::prelude::*;
use bevy_color::palettes::basic::{GREEN, RED, BLUE, YELLOW, ORANGE};
use bevy_gizmos::prelude::*;
use bevy_math::prelude::*;

/// System that sets up the camera for viewing gizmos.
fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5., 5., 5.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// System that renders various gizmos for demonstration purposes.
fn render_gizmos(mut gizmos: Gizmos) {
    // Draw a green ellipse at the origin
    gizmos.ellipse(Isometry3d::IDENTITY, Vec2::new(1., 2.), GREEN);

    // Draw a red ellipse with higher resolution (64 segments instead of default 32)
    gizmos
        .ellipse(Isometry3d::IDENTITY, Vec2::new(5., 1.), RED)
        .resolution(64);

    // Draw coordinate axes to help with orientation
    gizmos.ray(Vec3::ZERO, Vec3::X, BLUE);
    gizmos.ray(Vec3::ZERO, Vec3::Y, GREEN);
    gizmos.ray(Vec3::ZERO, Vec3::Z, RED);

    // Draw a sphere wireframe
    gizmos.sphere(Isometry3d::new(Vec3::new(3., 0., 0.), Quat::IDENTITY), 0.5, YELLOW);

    // Draw a cube wireframe
    gizmos.cuboid(
        Isometry3d::new(Vec3::new(-3., 0., 0.), Quat::IDENTITY),
        ORANGE,
    );

    // Draw some lines to connect points
    gizmos.line(Vec3::new(-2., 1., 0.), Vec3::new(2., 1., 0.), BLUE);
    gizmos.line(Vec3::new(-2., -1., 0.), Vec3::new(2., -1., 0.), BLUE);

    // Draw a circle in the XY plane
    gizmos.circle_2d(Isometry2d::IDENTITY, 2.0, YELLOW);

    // Draw some arrows for direction visualization
    gizmos.arrow(Vec3::new(0., 3., 0.), Vec3::new(1., 0., 0.), RED);
    gizmos.arrow(Vec3::new(0., 3., 0.), Vec3::new(0., 1., 0.), GREEN);
    gizmos.arrow(Vec3::new(0., 3., 0.), Vec3::new(0., 0., 1.), BLUE);
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_camera)
        .add_systems(Update, render_gizmos)
        .run();
}
