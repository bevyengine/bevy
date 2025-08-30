//! Demonstrates the behavior of the built-in easing functions.

use bevy::prelude::*;

#[derive(Component)]
#[require(Visibility, Transform)]
struct EaseFunctionPlot(EaseFunction, Color);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, display_curves)
        .run();
}

const COLS: usize = 12;
const EXTENT: Vec2 = Vec2::new(1172.0, 520.0);
const PLOT_SIZE: Vec2 = Vec2::splat(80.0);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let text_font = TextFont {
        font_size: 10.0,
        ..default()
    };

    let chunks = [
        // "In" row
        EaseFunction::SineIn,
        EaseFunction::QuadraticIn,
        EaseFunction::CubicIn,
        EaseFunction::QuarticIn,
        EaseFunction::QuinticIn,
        EaseFunction::SmoothStepIn,
        EaseFunction::SmootherStepIn,
        EaseFunction::CircularIn,
        EaseFunction::ExponentialIn,
        EaseFunction::ElasticIn,
        EaseFunction::BackIn,
        EaseFunction::BounceIn,
        // "Out" row
        EaseFunction::SineOut,
        EaseFunction::QuadraticOut,
        EaseFunction::CubicOut,
        EaseFunction::QuarticOut,
        EaseFunction::QuinticOut,
        EaseFunction::SmoothStepOut,
        EaseFunction::SmootherStepOut,
        EaseFunction::CircularOut,
        EaseFunction::ExponentialOut,
        EaseFunction::ElasticOut,
        EaseFunction::BackOut,
        EaseFunction::BounceOut,
        // "InOut" row
        EaseFunction::SineInOut,
        EaseFunction::QuadraticInOut,
        EaseFunction::CubicInOut,
        EaseFunction::QuarticInOut,
        EaseFunction::QuinticInOut,
        EaseFunction::SmoothStep,
        EaseFunction::SmootherStep,
        EaseFunction::CircularInOut,
        EaseFunction::ExponentialInOut,
        EaseFunction::ElasticInOut,
        EaseFunction::BackInOut,
        EaseFunction::BounceInOut,
        // "Other" row
        EaseFunction::Linear,
        EaseFunction::Steps(4, JumpAt::End),
        EaseFunction::Steps(4, JumpAt::Start),
        EaseFunction::Steps(4, JumpAt::Both),
        EaseFunction::Steps(4, JumpAt::None),
        EaseFunction::Elastic(50.0),
    ]
    .chunks(COLS);

    let max_rows = chunks.clone().count();

    let half_extent = EXTENT / 2.;
    let half_size = PLOT_SIZE / 2.;

    for (row, functions) in chunks.enumerate() {
        for (col, function) in functions.iter().enumerate() {
            let color = Hsla::hsl(col as f32 / COLS as f32 * 360.0, 0.8, 0.75).into();
            commands
                .spawn((
                    EaseFunctionPlot(*function, color),
                    Transform::from_xyz(
                        -half_extent.x + EXTENT.x / (COLS - 1) as f32 * col as f32,
                        half_extent.y - EXTENT.y / (max_rows - 1) as f32 * row as f32,
                        0.0,
                    ),
                ))
                .with_children(|p| {
                    // Marks the y value on the right side of the plot
                    p.spawn((
                        Sprite::from_color(color, Vec2::splat(5.0)),
                        Transform::from_xyz(half_size.x + 5.0, -half_size.y, 0.0),
                    ));
                    // Marks the x and y value inside the plot
                    p.spawn((
                        Sprite::from_color(color, Vec2::splat(4.0)),
                        Transform::from_xyz(-half_size.x, -half_size.y, 0.0),
                    ));

                    // Label
                    p.spawn((
                        Text2d(format!("{function:?}")),
                        text_font.clone(),
                        TextColor(color),
                        Transform::from_xyz(0.0, -half_size.y - 15.0, 0.0),
                    ));
                });
        }
    }
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn display_curves(
    mut gizmos: Gizmos,
    ease_functions: Query<(&EaseFunctionPlot, &Transform, &Children)>,
    mut transforms: Query<&mut Transform, Without<EaseFunctionPlot>>,
    mut ui_text: Single<&mut Text>,
    time: Res<Time>,
) {
    let samples = 100;
    let duration = 2.5;
    let time_margin = 0.5;

    let now = ((time.elapsed_secs() % (duration + time_margin * 2.0) - time_margin) / duration)
        .clamp(0.0, 1.0);

    ui_text.0 = format!("Progress: {now:.2}");

    for (EaseFunctionPlot(function, color), transform, children) in &ease_functions {
        let center = transform.translation.xy();
        let half_size = PLOT_SIZE / 2.0;

        // Draw a box around the curve
        gizmos.linestrip_2d(
            [
                center + half_size,
                center + half_size * Vec2::new(-1., 1.),
                center + half_size * Vec2::new(-1., -1.),
                center + half_size * Vec2::new(1., -1.),
                center + half_size,
            ],
            color.darker(0.4),
        );

        // Draw the curve
        let f = EasingCurve::new(0.0, 1.0, *function);
        let drawn_curve = f
            .by_ref()
            .graph()
            .map(|(x, y)| center - half_size + Vec2::new(x, y) * PLOT_SIZE);
        gizmos.curve_2d(
            &drawn_curve,
            drawn_curve.domain().spaced_points(samples).unwrap(),
            *color,
        );

        // Show progress along the curve for the current time
        let y = f.sample(now).unwrap() * PLOT_SIZE.y;
        transforms.get_mut(children[0]).unwrap().translation.y = -half_size.y + y;
        transforms.get_mut(children[1]).unwrap().translation =
            -half_size.extend(0.0) + Vec3::new(now * PLOT_SIZE.x, y, 0.0);

        // Show horizontal bar at y value
        gizmos.linestrip_2d(
            [
                center - half_size + Vec2::Y * y,
                center - half_size + Vec2::new(PLOT_SIZE.x, y),
            ],
            color.darker(0.2),
        );
    }
}
