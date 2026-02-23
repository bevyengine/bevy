//! Basic example demonstrating 3d text gizmos

use bevy::color::palettes::css::{ORANGE, RED, YELLOW};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_camera)
        .add_systems(Update, hello_world)
        .run();
}

fn setup_camera(mut commands: Commands, mut gizmo_config_store: ResMut<GizmoConfigStore>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    let (config, _) = gizmo_config_store.config_mut::<DefaultGizmoConfigGroup>();

    config.line.width = 4.;
}

fn hello_world(mut text_gizmos: Gizmos, time: Res<Time>) {
    let t = 0.2 * time.elapsed_secs();

    text_gizmos.text(
        Isometry3d::new(Vec3::new(0.0, 1.5, 0.0), Quat::from_rotation_y(-t)),
        "Hello",
        1.,
        Vec2::ZERO,
        RED,
    );

    text_gizmos.text(
        Isometry3d::new(Vec3::new(0.0, 0.0, 0.0), Quat::from_rotation_y(t + 0.25)),
        "Text",
        1.,
        Vec2::ZERO,
        ORANGE,
    );

    text_gizmos.text(
        Isometry3d::new(Vec3::new(0.0, -1.5, 0.0), Quat::from_rotation_y(-t - 0.5)),
        "Gizmos",
        1.,
        Vec2::ZERO,
        YELLOW,
    );
}
