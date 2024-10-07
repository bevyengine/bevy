//! Demonstrates the behavior of the built-in easing functions.

use bevy::{prelude::*, sprite::Anchor};

#[derive(Component)]
struct SelectedEaseFunction(easing::EaseFunction, Color);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, display_curves)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    let text_style = TextStyle {
        font_size: 10.0,
        ..default()
    };

    for (i, functions) in [
        easing::EaseFunction::QuadraticIn,
        easing::EaseFunction::QuadraticOut,
        easing::EaseFunction::QuadraticInOut,
        easing::EaseFunction::CubicIn,
        easing::EaseFunction::CubicOut,
        easing::EaseFunction::CubicInOut,
        easing::EaseFunction::QuarticIn,
        easing::EaseFunction::QuarticOut,
        easing::EaseFunction::QuarticInOut,
        easing::EaseFunction::QuinticIn,
        easing::EaseFunction::QuinticOut,
        easing::EaseFunction::QuinticInOut,
        easing::EaseFunction::CircularIn,
        easing::EaseFunction::CircularOut,
        easing::EaseFunction::CircularInOut,
        easing::EaseFunction::ExponentialIn,
        easing::EaseFunction::ExponentialOut,
        easing::EaseFunction::ExponentialInOut,
        easing::EaseFunction::SineIn,
        easing::EaseFunction::SineOut,
        easing::EaseFunction::SineInOut,
        easing::EaseFunction::ElasticIn,
        easing::EaseFunction::ElasticOut,
        easing::EaseFunction::ElasticInOut,
        easing::EaseFunction::BackIn,
        easing::EaseFunction::BackOut,
        easing::EaseFunction::BackInOut,
        easing::EaseFunction::BounceIn,
        easing::EaseFunction::BounceOut,
        easing::EaseFunction::BounceInOut,
    ]
    .chunks(3)
    .enumerate()
    {
        for j in 0..3 {
            let color = Hsla::hsl(i as f32 / 10.0 * 360.0, 0.8, 0.75).into();
            commands
                .spawn((
                    Text2dBundle {
                        text: Text::from_section(
                            format!("{:?}", functions[j]),
                            TextStyle {
                                color,
                                ..text_style.clone()
                            },
                        ),
                        transform: Transform::from_xyz(
                            i as f32 * 125.0 - 1280.0 / 2.0 + 25.0,
                            -100.0 - ((j as f32 * 250.0) - 300.0),
                            0.0,
                        ),
                        text_anchor: Anchor::TopLeft,
                        ..default()
                    },
                    SelectedEaseFunction(functions[j], color),
                ))
                .with_children(|p| {
                    p.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(5.0, 5.0)),
                            color,
                            ..default()
                        },
                        transform: Transform::from_xyz(110.0, 15.0, 0.0),
                        ..default()
                    });
                    p.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(4.0, 4.0)),
                            color,
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, 0.0, 0.0),
                        ..default()
                    });
                });
        }
    }
    commands.spawn(
        TextBundle::from_section("", TextStyle::default()).with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

fn display_curves(
    mut gizmos: Gizmos,
    ease_functions: Query<(&SelectedEaseFunction, &Transform, &Children)>,
    mut transforms: Query<&mut Transform, Without<SelectedEaseFunction>>,
    mut ui: Query<&mut Text, With<Node>>,
    time: Res<Time>,
) {
    let samples = 100;
    let size = 100.0;
    let duration = 2.5;
    let time_margin = 0.5;

    let now = ((time.elapsed_seconds() % (duration + time_margin * 2.0) - time_margin) / duration)
        .clamp(0.0, 1.0);

    ui.single_mut().sections[0].value = format!("Progress: {:.2}", now);

    for (SelectedEaseFunction(function, color), transform, children) in &ease_functions {
        // Draw a box around the curve
        gizmos.linestrip_2d(
            [
                Vec2::new(transform.translation.x, transform.translation.y + 15.0),
                Vec2::new(
                    transform.translation.x + size,
                    transform.translation.y + 15.0,
                ),
                Vec2::new(
                    transform.translation.x + size,
                    transform.translation.y + 15.0 + size,
                ),
                Vec2::new(
                    transform.translation.x,
                    transform.translation.y + 15.0 + size,
                ),
                Vec2::new(transform.translation.x, transform.translation.y + 15.0),
            ],
            color.darker(0.4),
        );

        // Draw the curve
        let f = easing::EasingCurve::ease(*function);
        gizmos.linestrip_2d(
            (0..(samples + 1)).map(|i| {
                let t = i as f32 / samples as f32;
                let sampled = f.sample(t).unwrap();
                Vec2::new(
                    t * size + transform.translation.x,
                    sampled * size + transform.translation.y + 15.0,
                )
            }),
            *color,
        );

        // Show progress along the curve for the current time
        let y = f.sample(now).unwrap() * size + 15.0;
        transforms.get_mut(children[0]).unwrap().translation.y = y;
        transforms.get_mut(children[1]).unwrap().translation = Vec3::new(now * size, y, 0.0);
        gizmos.linestrip_2d(
            [
                Vec2::new(transform.translation.x, transform.translation.y + y),
                Vec2::new(transform.translation.x + size, transform.translation.y + y),
            ],
            color.darker(0.2),
        );
    }
}
