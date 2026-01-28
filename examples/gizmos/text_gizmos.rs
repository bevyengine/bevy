//! Example demonstrating text gizmos

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_camera)
        .add_systems(Update, draw_hello)
        .run();
}

fn setup_camera(mut commands: Commands, mut config_store: ResMut<GizmoConfigStore>) {
    commands.spawn(Camera2d);

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 10.;
    config.line.joints = GizmoLineJoint::Round(4);
}

fn draw_hello(mut gizmos: Gizmos) {
    gizmos.text_2d_simplex(
        Isometry2d::from_translation(-400.0 * Vec2::X),
        "Hello, gizmos!\nSecond Line!",
        100.0,
        120.0,
        Color::WHITE,
    );
}
