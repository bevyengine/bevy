//! Simple example demonstrating linear gradients.

use std::f32::consts::PI;
use std::f32::consts::TAU;

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::LIGHT_CYAN;
use bevy::color::palettes::css::LIME;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::YELLOW;
use bevy::math::Rect;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::ui::ColorStop;
use bevy::ui::ColorStops;
use bevy::ui::LinearGradient;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_angles)
        .run();
}

fn big(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            width: Val::Px(1000.),
            height: Val::Px(500.),
            margin: UiRect::all(Val::Px(10.)),
            ..default()
        },
        Outline {
            width: Val::Px(1.),
            offset: Val::Px(1.),
            color: Color::WHITE,
        },
        LinearGradient { angle: TAU / 4. },
        ColorStops(vec![
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
        ]),
    ));
}

fn setup_single(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            width: Val::Px(100.),
            height: Val::Px(100.),
            margin: UiRect::all(Val::Px(50.)),
            ..default()
        },
        Outline {
            width: Val::Px(1.),
            offset: Val::Px(1.),
            color: Color::WHITE,
        },
        LinearGradient { angle: 0. },
        ColorStops(vec![
            ColorStop {
                color: RED.into(),
                point: Val::Auto,
            },
            ColorStop {
                color: BLUE.into(),
                point: Val::Auto,
            },
        ]),
    ));
}

fn setup_angles(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.),
            ..Default::default()
        })
        .with_children(|commands| {
            for stops in [
                ColorStops(vec![
                    ColorStop {
                        color: Color::WHITE,
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: Color::BLACK,
                        point: Val::Auto,
                    },
                ]),
                ColorStops(vec![
                    ColorStop {
                        color: RED.into(),
                        point: Val::Auto,
                    },
                    ColorStop {
                        color: BLUE.into(),
                        point: Val::Auto,
                    },
                ]),
                ColorStops(vec![
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
                ]),
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
                                Outline {
                                    width: Val::Px(1.),
                                    offset: Val::Px(1.),
                                    color: Color::WHITE,
                                },
                                LinearGradient { angle },
                                stops.clone(),
                            ));
                        }
                    });
                }
            }
        });
}

fn setup_angles_rgb(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(50.),
            ..Default::default()
        })
        .with_children(|commands| {
            for (w, h) in [(100., 100.), (100., 200.), (100., 50.)] {
                commands.spawn(Node::default()).with_children(|commands| {
                    for angle in (0..8).map(|i| i as f32 * std::f32::consts::TAU / 8.) {
                        commands.spawn((
                            Node {
                                width: Val::Px(w),
                                height: Val::Px(h),
                                margin: UiRect::all(Val::Px(10.)),
                                ..default()
                            },
                            Outline {
                                width: Val::Px(1.),
                                offset: Val::Px(1.),
                                color: Color::WHITE,
                            },
                            LinearGradient { angle },
                            ColorStops(vec![
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
                            ]),
                        ));
                    }
                });
            }
        });
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn(Node::default()).with_children(|commands| {
        commands.spawn((
            Node {
                width: Val::Px(100.),
                height: Val::Px(100.),
                margin: UiRect::all(Val::Px(50.)),
                ..default()
            },
            Outline {
                width: Val::Px(1.),
                offset: Val::Px(1.),
                color: Color::WHITE,
            },
            LinearGradient { angle: 0. },
            ColorStops(vec![
                ColorStop {
                    color: RED.into(),
                    point: Val::Auto,
                },
                ColorStop {
                    color: BLUE.into(),
                    point: Val::Auto,
                },
            ]),
        ));

        commands.spawn((
            Node {
                width: Val::Px(100.),
                height: Val::Px(100.),
                margin: UiRect::all(Val::Px(50.)),
                ..default()
            },
            Outline {
                width: Val::Px(1.),
                offset: Val::Px(1.),
                color: Color::WHITE,
            },
            LinearGradient { angle: 0. },
            ColorStops(vec![
                ColorStop {
                    color: RED.into(),
                    point: Val::Auto,
                },
                ColorStop {
                    color: RED.into(),
                    point: Val::Auto,
                },
                ColorStop {
                    color: GREEN.into(),
                    point: Val::Auto,
                },
                ColorStop {
                    color: GREEN.into(),
                    point: Val::Auto,
                },
                ColorStop {
                    color: BLUE.into(),
                    point: Val::Auto,
                },
                ColorStop {
                    color: BLUE.into(),
                    point: Val::Auto,
                },
            ]),
        ));
    });
}

fn setup_2(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            row_gap: Val::Px(5.),
            column_gap: Val::Px(5.),
            ..Default::default()
        })
        .with_children(|commands| {
            commands.spawn((
                Node {
                    width: Val::Px(100.),
                    height: Val::Px(200.),
                    ..default()
                },
                LinearGradient { angle: 0. },
                ColorStops(vec![
                    ColorStop {
                        color: YELLOW.into(),
                        point: Val::ZERO,
                    },
                    ColorStop {
                        color: BLUE.into(),
                        point: Val::Px(100.),
                    },
                    ColorStop {
                        color: GREEN.into(),
                        point: Val::Px(50.),
                    },
                ]),
            ));

            commands.spawn((
                Node {
                    width: Val::Px(100.),
                    height: Val::Px(200.),
                    ..default()
                },
                LinearGradient { angle: 0. },
                ColorStops(vec![
                    ColorStop {
                        color: YELLOW.into(),
                        point: Val::Px(200.),
                    },
                    ColorStop {
                        color: BLUE.into(),
                        point: Val::Px(100.),
                    },
                    ColorStop {
                        color: GREEN.into(),
                        point: Val::Px(50.),
                    },
                ]),
            ));

            commands.spawn((
                Node {
                    width: Val::Px(100.),
                    height: Val::Px(200.),
                    ..default()
                },
                LinearGradient { angle: 0. },
                ColorStops(vec![
                    ColorStop {
                        color: YELLOW.into(),
                        point: Val::Px(10.),
                    },
                    ColorStop {
                        color: BLUE.into(),
                        point: Val::Px(20.),
                    },
                    ColorStop {
                        color: GREEN.into(),
                        point: Val::Px(30.),
                    },
                ]),
            ));

            commands.spawn((
                Node {
                    width: Val::Px(100.),
                    height: Val::Px(200.),
                    ..default()
                },
                BackgroundColor(LIGHT_CYAN.into()),
            ));
        });
}
