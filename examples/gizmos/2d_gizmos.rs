//! This example demonstrates Bevy's immediate mode drawing API intended for visual debugging.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::{color::palettes::css::*, math::Isometry2d, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_gizmo_group::<MyRoundGizmos>()
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_example_collection, update_config))
        .run();
}

// We can create our own gizmo config group!
#[derive(Default, Reflect, GizmoConfigGroup)]
struct MyRoundGizmos {}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    // text
    commands.spawn((
        Text::new(
            "Hold 'Left' or 'Right' to change the line width of straight gizmos\n\
        Hold 'Up' or 'Down' to change the line width of round gizmos\n\
        Press '1' / '2' to toggle the visibility of straight / round gizmos\n\
        Press 'U' / 'I' to cycle through line styles\n\
        Press 'J' / 'K' to cycle through line joins",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.),
            left: Val::Px(12.),
            ..default()
        },
    ));
}

fn draw_example_collection(
    mut gizmos: Gizmos,
    mut my_gizmos: Gizmos<MyRoundGizmos>,
    time: Res<Time>,
) {
    let sin_t_scaled = ops::sin(time.elapsed_secs()) * 50.;
    gizmos.line_2d(Vec2::Y * -sin_t_scaled, Vec2::splat(-80.), RED);
    gizmos.ray_2d(Vec2::Y * sin_t_scaled, Vec2::splat(80.), LIME);

    gizmos
        .grid_2d(
            Isometry2d::IDENTITY,
            UVec2::new(16, 9),
            Vec2::new(80., 80.),
            // Dark gray
            LinearRgba::gray(0.05),
        )
        .outer_edges();

    // Triangle
    gizmos.linestrip_gradient_2d([
        (Vec2::Y * 300., BLUE),
        (Vec2::new(-255., -155.), RED),
        (Vec2::new(255., -155.), LIME),
        (Vec2::Y * 300., BLUE),
    ]);

    gizmos.rect_2d(Isometry2d::IDENTITY, Vec2::splat(650.), BLACK);

    gizmos.cross_2d(Vec2::new(-160., 120.), 12., FUCHSIA);

    let domain = Interval::EVERYWHERE;
    let curve = FunctionCurve::new(domain, |t| Vec2::new(t, ops::sin(t / 25.0) * 100.0));
    let resolution = ((ops::sin(time.elapsed_secs()) + 1.0) * 50.0) as usize;
    let times_and_colors = (0..=resolution)
        .map(|n| n as f32 / resolution as f32)
        .map(|t| (t - 0.5) * 600.0)
        .map(|t| (t, TEAL.mix(&HOT_PINK, (t + 300.0) / 600.0)));
    gizmos.curve_gradient_2d(curve, times_and_colors);

    my_gizmos
        .rounded_rect_2d(Isometry2d::IDENTITY, Vec2::splat(630.), BLACK)
        .corner_radius(ops::cos(time.elapsed_secs() / 3.) * 100.);

    // Circles have 32 line-segments by default.
    // You may want to increase this for larger circles.
    my_gizmos
        .circle_2d(Isometry2d::IDENTITY, 300., NAVY)
        .resolution(64);

    my_gizmos.ellipse_2d(
        Rot2::radians(time.elapsed_secs() % TAU),
        Vec2::new(100., 200.),
        YELLOW_GREEN,
    );

    // Arcs default resolution is linearly interpolated between
    // 1 and 32, using the arc length as scalar.
    my_gizmos.arc_2d(
        Rot2::radians(sin_t_scaled / 10.),
        FRAC_PI_2,
        310.,
        ORANGE_RED,
    );
    my_gizmos.arc_2d(Isometry2d::IDENTITY, FRAC_PI_2, 80.0, ORANGE_RED);
    my_gizmos.long_arc_2d_between(Vec2::ZERO, Vec2::X * 20.0, Vec2::Y * 20.0, ORANGE_RED);
    my_gizmos.short_arc_2d_between(Vec2::ZERO, Vec2::X * 40.0, Vec2::Y * 40.0, ORANGE_RED);

    gizmos.arrow_2d(
        Vec2::ZERO,
        Vec2::from_angle(sin_t_scaled / -10. + PI / 2.) * 50.,
        YELLOW,
    );

    // You can create more complex arrows using the arrow builder.
    gizmos
        .arrow_2d(
            Vec2::ZERO,
            Vec2::from_angle(sin_t_scaled / -10.) * 50.,
            GREEN,
        )
        .with_double_end()
        .with_tip_length(10.);
}

fn update_config(
    mut config_store: ResMut<GizmoConfigStore>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    if keyboard.pressed(KeyCode::ArrowRight) {
        config.line.width += 5. * time.delta_secs();
        config.line.width = config.line.width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        config.line.width -= 5. * time.delta_secs();
        config.line.width = config.line.width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit1) {
        config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyU) {
        config.line.style = match config.line.style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            GizmoLineStyle::Dotted => GizmoLineStyle::Dashed {
                gap_scale: 3.0,
                line_scale: 5.0,
            },
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyJ) {
        config.line.joints = match config.line.joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    let (my_config, _) = config_store.config_mut::<MyRoundGizmos>();
    if keyboard.pressed(KeyCode::ArrowUp) {
        my_config.line.width += 5. * time.delta_secs();
        my_config.line.width = my_config.line.width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        my_config.line.width -= 5. * time.delta_secs();
        my_config.line.width = my_config.line.width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        my_config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyI) {
        my_config.line.style = match my_config.line.style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            GizmoLineStyle::Dotted => GizmoLineStyle::Dashed {
                gap_scale: 3.0,
                line_scale: 5.0,
            },
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyK) {
        my_config.line.joints = match my_config.line.joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }
}
