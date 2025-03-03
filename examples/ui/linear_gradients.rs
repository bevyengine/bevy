//! Simple example demonstrating linear gradients.

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::LIME;
use bevy::color::palettes::css::ORANGE;
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
        .add_systems(Update, update)
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
            ] {
                commands.spawn(Node::default()).with_children(|commands| {
                    commands
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        })
                        .with_children(|commands| {
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
                                width: Val::Px(200.),
                                height: Val::Px(200.),
                                border: UiRect::all(Val::Px(5.)),
                                margin: UiRect::all(Val::Px(10.)),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(20.)),
                            LinearGradient {
                                angle: 0.,
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

fn update(time: Res<Time>, mut query: Query<&mut LinearGradient, With<AnimateMarker>>) {
    for mut gradient in query.iter_mut() {
        gradient.angle += 0.5 * time.delta_secs();
    }
}
