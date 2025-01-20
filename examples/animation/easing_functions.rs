//! Demonstrates the behavior of the built-in easing functions.

use bevy::{prelude::*, sprite::Anchor};

#[derive(Component)]
struct SelectedEaseFunction(EaseFunction, Color);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, display_curves)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let text_font = TextFont {
        font_size: 10.0,
        ..default()
    };

    for (i, functions) in [
        EaseFunction::SineIn,
        EaseFunction::SineOut,
        EaseFunction::SineInOut,
        EaseFunction::QuadraticIn,
        EaseFunction::QuadraticOut,
        EaseFunction::QuadraticInOut,
        EaseFunction::CubicIn,
        EaseFunction::CubicOut,
        EaseFunction::CubicInOut,
        EaseFunction::QuarticIn,
        EaseFunction::QuarticOut,
        EaseFunction::QuarticInOut,
        EaseFunction::QuinticIn,
        EaseFunction::QuinticOut,
        EaseFunction::QuinticInOut,
        EaseFunction::CircularIn,
        EaseFunction::CircularOut,
        EaseFunction::CircularInOut,
        EaseFunction::ExponentialIn,
        EaseFunction::ExponentialOut,
        EaseFunction::ExponentialInOut,
        EaseFunction::ElasticIn,
        EaseFunction::ElasticOut,
        EaseFunction::ElasticInOut,
        EaseFunction::BackIn,
        EaseFunction::BackOut,
        EaseFunction::BackInOut,
        EaseFunction::BounceIn,
        EaseFunction::BounceOut,
        EaseFunction::BounceInOut,
        EaseFunction::Linear,
        EaseFunction::Steps(4),
        EaseFunction::Elastic(50.0),
    ]
    .chunks(3)
    .enumerate()
    {
        for (j, function) in functions.iter().enumerate() {
            let color = Hsla::hsl(i as f32 / 11.0 * 360.0, 0.8, 0.75).into();
            commands
                .spawn((
                    Text2d(format!("{:?}", function)),
                    text_font.clone(),
                    TextColor(color),
                    Transform::from_xyz(
                        i as f32 * 113.0 - 1280.0 / 2.0 + 25.0,
                        -100.0 - ((j as f32 * 250.0) - 300.0),
                        0.0,
                    ),
                    Anchor::TopLeft,
                    SelectedEaseFunction(*function, color),
                ))
                .with_children(|p| {
                    p.spawn((
                        Sprite::from_color(color, Vec2::splat(5.0)),
                        Transform::from_xyz(SIZE_PER_FUNCTION + 5.0, 15.0, 0.0),
                    ));
                    p.spawn((
                        Sprite::from_color(color, Vec2::splat(4.0)),
                        Transform::from_xyz(0.0, 0.0, 0.0),
                    ));
                });
        }
    }
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

const SIZE_PER_FUNCTION: f32 = 95.0;

fn display_curves(
    mut gizmos: Gizmos,
    ease_functions: Query<(&SelectedEaseFunction, &Transform, &Children)>,
    mut transforms: Query<&mut Transform, Without<SelectedEaseFunction>>,
    mut ui_text: Single<&mut Text>,
    time: Res<Time>,
) {
    let samples = 100;
    let duration = 2.5;
    let time_margin = 0.5;

    let now = ((time.elapsed_secs() % (duration + time_margin * 2.0) - time_margin) / duration)
        .clamp(0.0, 1.0);

    ui_text.0 = format!("Progress: {:.2}", now);

    for (SelectedEaseFunction(function, color), transform, children) in &ease_functions {
        // Draw a box around the curve
        gizmos.linestrip_2d(
            [
                Vec2::new(transform.translation.x, transform.translation.y + 15.0),
                Vec2::new(
                    transform.translation.x + SIZE_PER_FUNCTION,
                    transform.translation.y + 15.0,
                ),
                Vec2::new(
                    transform.translation.x + SIZE_PER_FUNCTION,
                    transform.translation.y + 15.0 + SIZE_PER_FUNCTION,
                ),
                Vec2::new(
                    transform.translation.x,
                    transform.translation.y + 15.0 + SIZE_PER_FUNCTION,
                ),
                Vec2::new(transform.translation.x, transform.translation.y + 15.0),
            ],
            color.darker(0.4),
        );

        // Draw the curve
        let f = EasingCurve::new(0.0, 1.0, *function);
        let drawn_curve = f.by_ref().graph().map(|(x, y)| {
            Vec2::new(
                x * SIZE_PER_FUNCTION + transform.translation.x,
                y * SIZE_PER_FUNCTION + transform.translation.y + 15.0,
            )
        });
        gizmos.curve_2d(
            &drawn_curve,
            drawn_curve.domain().spaced_points(samples).unwrap(),
            *color,
        );

        // Show progress along the curve for the current time
        let y = f.sample(now).unwrap() * SIZE_PER_FUNCTION + 15.0;
        transforms.get_mut(children[0]).unwrap().translation.y = y;
        transforms.get_mut(children[1]).unwrap().translation =
            Vec3::new(now * SIZE_PER_FUNCTION, y, 0.0);
        gizmos.linestrip_2d(
            [
                Vec2::new(transform.translation.x, transform.translation.y + y),
                Vec2::new(
                    transform.translation.x + SIZE_PER_FUNCTION,
                    transform.translation.y + y,
                ),
            ],
            color.darker(0.2),
        );
    }
}
