//! Simple example demonstrating linear gradients.

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::LIME;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;
use bevy::ui::ColorStop;
use bevy::ui::LinearGradient;
use std::f32::consts::TAU;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.),
            ..Default::default()
        })
        .with_children(|commands| {
            for stops in [
                vec![
                    ColorStop {
                        color: Color::WHITE,
                        point: Val::Percent(10.),
                    },
                    ColorStop {
                        color: Color::BLACK,
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: Color::WHITE,
                        point: Val::Percent(90.),
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
                ],
                vec![
                    ColorStop {
                        color: RED.into(),
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: LIME.into(),
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: BLUE.into(),
                        point: Val::Auto,
                    },
                ],
            ] {
                for (w, h) in [(100., 100.), (50., 100.), (100., 50.)] {
                    commands.spawn(Node::default()).with_children(|commands| {
                        for angle in (0..8).map(|i| i as f32 * TAU / 8.) {
                            commands.spawn((
                                Node {
                                    width: Val::Px(w),
                                    height: Val::Px(h),
                                    border: UiRect::all(Val::Px(5.)),
                                    margin: UiRect::all(Val::Px(10.)),
                                    ..default()
                                },
                                BorderRadius::all(Val::Px(20.)),
                                Outline {
                                    width: Val::Px(1.),
                                    offset: Val::Px(1.),
                                    color: Color::WHITE,
                                },
                                LinearGradient {
                                    angle,
                                    stops: stops.clone(),
                                },
                                LinearGradientBorder(LinearGradient {
                                    angle: 3. * TAU / 8.,
                                    stops: vec![
                                        ColorStop {
                                            color: YELLOW.into(),
                                            point: Val::Auto,
                                        },
                                        Color::WHITE.into(),
                                    ],
                                }),
                            ));
                        }
                    });
                }
            }
        });
}

fn setup_border(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            ..Default::default()
        })
        .with_children(|commands| {
            commands.spawn((
                Node {
                    width: Val::Px(500.),
                    height: Val::Px(500.),
                    border: UiRect::all(Val::Px(50.)),
                    margin: UiRect::all(Val::Px(10.)),
                    ..default()
                },
                BorderRadius::all(Val::Px(20.)),
                Outline {
                    width: Val::Px(1.),
                    offset: Val::Px(1.),
                    color: Color::WHITE,
                },
                LinearGradientBorder(LinearGradient {
                    angle: 3. * TAU / 8.,
                    stops: vec![
                        ColorStop {
                            color: YELLOW.into(),
                            point: Val::Auto,
                        },
                        Color::WHITE.into(),
                    ],
                }),
            ));
        });
}
