//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.

use std::f32::consts::PI;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_gizmo_config::<MyGizmos>()
        .add_systems(Startup, setup)
        .add_systems(Update, (system, update_config))
        .run();
}

// We can create our own gizmo config!
#[derive(Default, Clone, TypePath)]
struct MyGizmos {}

impl CustomGizmoConfig for MyGizmos {}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    // text
    commands.spawn(TextBundle::from_section(
        "Hold 'Left' or 'Right' to change the line width of straight gizmos\n\
        Hold 'Up' or 'Down' to change the line width of round gizmos\n\
        Press '1' or '2' to toggle the visibility of straight gizmos or round gizmos",
        TextStyle {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            font_size: 24.,
            color: Color::WHITE,
        },
    ));
}

fn system(mut gizmos: Gizmos, mut my_gizmos: Gizmos<MyGizmos>, time: Res<Time>) {
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
    my_gizmos.circle_2d(Vec2::ZERO, 120., Color::BLACK);
    // You may want to increase this for larger circles.
    my_gizmos.circle_2d(Vec2::ZERO, 300., Color::NAVY).segments(64);

    // Arcs default amount of segments is linearly interpolated between
    // 1 and 32, using the arc length as scalar.
    my_gizmos.arc_2d(Vec2::ZERO, sin / 10., PI / 2., 350., Color::ORANGE_RED);
}

fn update_config(
    mut config: ResMut<GizmoConfig>,
    mut my_config: ResMut<GizmoConfig<MyGizmos>>,
    keyboard: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    if keyboard.pressed(KeyCode::Right) {
        config.line_width += 5. * time.delta_seconds();
        config.line_width = config.line_width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::Left) {
        config.line_width -= 5. * time.delta_seconds();
        config.line_width = config.line_width.clamp(0., 50.);
    }

    if keyboard.pressed(KeyCode::Up) {
        my_config.line_width += 5. * time.delta_seconds();
        my_config.line_width = my_config.line_width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::Down) {
        my_config.line_width -= 5. * time.delta_seconds();
        my_config.line_width = my_config.line_width.clamp(0., 50.);
    }

    if keyboard.just_pressed(KeyCode::Key1) {
        config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::Key2) {
        my_config.enabled ^= true;
    }
}
