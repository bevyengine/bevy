//! Simple example demonstrating linear gradients.

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::INDIGO;
use bevy::color::palettes::css::LIME;
use bevy::color::palettes::css::ORANGE;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::VIOLET;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;
use bevy::ui::ColorStop;
use std::f32::consts::TAU;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(30.),
            margin: UiRect::all(Val::Px(30.)),
            ..Default::default()
        })
        .with_children(|commands| {
            for (b, stops) in [
                (
                    5.,
                    vec![
                        ColorStop {
                            color: Color::WHITE,
                            point: Val::Percent(15.),
                        },
                        ColorStop {
                            color: Color::BLACK,
                            point: Val::Percent(85.),
                        },
                    ],
                ),
                (
                    5.,
                    vec![
                        ColorStop {
                            color: RED.into(),
                            point: Val::Auto,
                        },
                        ColorStop {
                            color: BLUE.into(),
                            point: Val::Auto,
                        },
                        ColorStop {
                            color: LIME.into(),
                            point: Val::Auto,
                        },
                    ],
                ),
                (
                    0.,
                    vec![
                        ColorStop {
                            color: RED.into(),
                            point: Val::Auto,
                        },
                        ColorStop {
                            color: RED.into(),
                            point: Val::Percent(100. / 7.),
                        },
                        ColorStop {
                            color: ORANGE.into(),
                            point: Val::Percent(100. / 7.),
                        },
                        ColorStop {
                            color: ORANGE.into(),
                            point: Val::Percent(200. / 7.),
                        },
                        ColorStop {
                            color: YELLOW.into(),
                            point: Val::Percent(200. / 7.),
                        },
                        ColorStop {
                            color: YELLOW.into(),
                            point: Val::Percent(300. / 7.),
                        },
                        ColorStop {
                            color: GREEN.into(),
                            point: Val::Percent(300. / 7.),
                        },
                        ColorStop {
                            color: GREEN.into(),
                            point: Val::Percent(400. / 7.),
                        },
                        ColorStop {
                            color: BLUE.into(),
                            point: Val::Percent(400. / 7.),
                        },
                        ColorStop {
                            color: BLUE.into(),
                            point: Val::Percent(500. / 7.),
                        },
                        ColorStop {
                            color: INDIGO.into(),
                            point: Val::Percent(500. / 7.),
                        },
                        ColorStop {
                            color: INDIGO.into(),
                            point: Val::Percent(600. / 7.),
                        },
                        ColorStop {
                            color: VIOLET.into(),
                            point: Val::Percent(600. / 7.),
                        },
                        ColorStop {
                            color: VIOLET.into(),
                            point: Val::Auto,
                        },
                    ],
                ),
            ] {
                commands.spawn(Node::default()).with_children(|commands| {
                    commands
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(10.),
                            ..Default::default()
                        })
                        .with_children(|commands| {
                            for (w, h) in [(100., 100.), (50., 100.), (100., 50.)] {
                                commands
                                    .spawn(Node {
                                        column_gap: Val::Px(10.),
                                        ..Default::default()
                                    })
                                    .with_children(|commands| {
                                        for angle in (0..8).map(|i| i as f32 * TAU / 8.) {
                                            commands.spawn((
                                                Node {
                                                    width: Val::Px(w),
                                                    height: Val::Px(h),
                                                    border: UiRect::all(Val::Px(b)),
                                                    ..default()
                                                },
                                                BorderRadius::all(Val::Px(20.)),
                                                BackgroundGradient::from(Gradient::Linear {
                                                    angle,
                                                    stops: stops.clone(),
                                                }),
                                                BorderGradient::from(Gradient::Linear {
                                                    angle: 3. * TAU / 8.,
                                                    stops: vec![
                                                        ColorStop {
                                                            color: YELLOW.into(),
                                                            point: Val::Auto,
                                                        },
                                                        Color::WHITE.into(),
                                                        ColorStop {
                                                            color: ORANGE.into(),
                                                            point: Val::Auto,
                                                        },
                                                    ],
                                                }),
                                            ));
                                        }
                                    });
                            }
                        });

                    commands.spawn(Node::default()).with_children(|commands| {
                        commands.spawn((
                            Node {
                                aspect_ratio: Some(1.),
                                height: Val::Percent(100.),
                                border: UiRect::all(Val::Px(b)),
                                margin: UiRect::left(Val::Px(30.)),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(20.)),
                            BackgroundGradient::from(Gradient::Linear {
                                angle: 0.,
                                stops: stops.clone(),
                            }),
                            BorderGradient::from(Gradient::Linear {
                                angle: 3. * TAU / 8.,
                                stops: vec![
                                    ColorStop {
                                        color: YELLOW.into(),
                                        point: Val::Auto,
                                    },
                                    Color::WHITE.into(),
                                    ColorStop {
                                        color: ORANGE.into(),
                                        point: Val::Auto,
                                    },
                                ],
                            }),
                            AnimateMarker,
                        ));

                        commands.spawn((
                            Node {
                                aspect_ratio: Some(1.),
                                height: Val::Percent(100.),
                                border: UiRect::all(Val::Px(b)),
                                margin: UiRect::left(Val::Px(30.)),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(20.)),
                            BackgroundGradient::from(Gradient::Radial {
                                stops: stops.clone(),
                                position: RelativePosition::center(Val::Px(25.), Val::Px(25.)),
                                shape: RadialGradientShape::ClosestSide,
                            }),
                            BorderGradient::from(Gradient::Linear {
                                angle: 3. * TAU / 8.,
                                stops: vec![
                                    ColorStop {
                                        color: YELLOW.into(),
                                        point: Val::Auto,
                                    },
                                    Color::WHITE.into(),
                                    ColorStop {
                                        color: ORANGE.into(),
                                        point: Val::Auto,
                                    },
                                ],
                            }),
                            AnimateMarker,
                        ));
                        commands.spawn((
                            Node {
                                aspect_ratio: Some(1.),
                                height: Val::Percent(100.),
                                border: UiRect::all(Val::Px(b)),
                                margin: UiRect::left(Val::Px(30.)),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(20.)),
                            BackgroundGradient::from(Gradient::Conic {
                                stops: stops
                                    .clone()
                                    .into_iter()
                                    .map(|stop| AngularColorStop {
                                        color: stop.color,
                                        angle: None,
                                    })
                                    .collect(),
                                position: RelativePosition::CENTER,
                            }),
                            BorderGradient::from(Gradient::Linear {
                                angle: 3. * TAU / 8.,
                                stops: vec![
                                    ColorStop {
                                        color: YELLOW.into(),
                                        point: Val::Auto,
                                    },
                                    Color::WHITE.into(),
                                    ColorStop {
                                        color: ORANGE.into(),
                                        point: Val::Auto,
                                    },
                                ],
                            }),
                            AnimateMarker,
                        ));
                    });
                });
            }
        });
}

#[derive(Component)]
struct AnimateMarker;

fn update(time: Res<Time>, mut query: Query<&mut BackgroundGradient, With<AnimateMarker>>) {
    for mut gradients in query.iter_mut() {
        for gradient in gradients.0.iter_mut() {
            if let Gradient::Linear { angle, .. } = gradient {
                *angle += 0.5 * time.delta_secs();
            }
        }
    }
}
