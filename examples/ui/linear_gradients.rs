//! Simple example demonstrating linear gradients.

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::DARK_CYAN;
use bevy::color::palettes::css::DARK_GOLDENROD;
use bevy::color::palettes::css::LIGHT_CYAN;
use bevy::color::palettes::css::LIME;
use bevy::color::palettes::css::MAGENTA;
use bevy::color::palettes::css::ORANGE;
use bevy::color::palettes::css::PURPLE;
use bevy::color::palettes::css::RED;
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
            for stops in [
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
                vec![
                    ColorStop {
                        color: PURPLE.into(),
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: DARK_GOLDENROD.into(),
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: YELLOW.into(),
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: BLUE.into(),
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: MAGENTA.into(),
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: LIGHT_CYAN.into(),
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: DARK_CYAN.into(),
                        point: Val::Auto,
                    },
                ],
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
                                                    border: UiRect::all(Val::Px(5.)),
                                                    ..default()
                                                },
                                                BorderRadius::all(Val::Px(20.)),
                                                GradientNode(Gradient::Linear {
                                                    angle,
                                                    stops: stops.clone(),
                                                }),
                                                GradientBorder(Gradient::Linear {
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
                                border: UiRect::all(Val::Px(5.)),
                                margin: UiRect::left(Val::Px(30.)),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(20.)),
                            GradientNode(Gradient::Linear {
                                angle: 0.,
                                stops: stops.clone(),
                            }),
                            GradientBorder(Gradient::Linear {
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

fn update(time: Res<Time>, mut query: Query<&mut GradientNode, With<AnimateMarker>>) {
    for mut gradient in query.iter_mut() {
        if let Gradient::Linear { angle, .. } = &mut gradient.0 {
            *angle += 0.5 * time.delta_secs();
        }
    }
}
