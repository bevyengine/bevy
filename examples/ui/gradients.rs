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
                        ColorStop::new(Color::WHITE, Val::Percent(15.)),
                        ColorStop::new(Color::BLACK, Val::Percent(85.)),
                    ],
                ),
                (5., vec![RED.into(), BLUE.into(), LIME.into()]),
                (
                    0.,
                    vec![
                        RED.into(),
                        ColorStop::new(RED, Val::Percent(100. / 7.)),
                        ColorStop::new(ORANGE, Val::Percent(100. / 7.)),
                        ColorStop::new(ORANGE, Val::Percent(200. / 7.)),
                        ColorStop::new(YELLOW, Val::Percent(200. / 7.)),
                        ColorStop::new(YELLOW, Val::Percent(300. / 7.)),
                        ColorStop::new(GREEN, Val::Percent(300. / 7.)),
                        ColorStop::new(GREEN, Val::Percent(400. / 7.)),
                        ColorStop::new(BLUE, Val::Percent(400. / 7.)),
                        ColorStop::new(BLUE, Val::Percent(500. / 7.)),
                        ColorStop::new(INDIGO, Val::Percent(500. / 7.)),
                        ColorStop::new(INDIGO, Val::Percent(600. / 7.)),
                        ColorStop::new(VIOLET, Val::Percent(600. / 7.)),
                        VIOLET.into(),
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
                                                BackgroundGradient::from(LinearGradient {
                                                    angle,
                                                    stops: stops.clone(),
                                                }),
                                                BorderGradient::from(LinearGradient {
                                                    angle: 3. * TAU / 8.,
                                                    stops: vec![
                                                        YELLOW.into(),
                                                        Color::WHITE.into(),
                                                        ORANGE.into(),
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
                            BackgroundGradient::from(LinearGradient {
                                angle: 0.,
                                stops: stops.clone(),
                            }),
                            BorderGradient::from(LinearGradient {
                                angle: 3. * TAU / 8.,
                                stops: vec![YELLOW.into(), Color::WHITE.into(), ORANGE.into()],
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
                            BackgroundGradient::from(RadialGradient {
                                stops: stops.clone(),
                                shape: RadialGradientShape::ClosestSide,
                                position: Position::CENTER,
                            }),
                            BorderGradient::from(LinearGradient {
                                angle: 3. * TAU / 8.,
                                stops: vec![YELLOW.into(), Color::WHITE.into(), ORANGE.into()],
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
                            BackgroundGradient::from(ConicGradient {
                                start: 0.,
                                stops: stops
                                    .iter()
                                    .map(|stop| AngularColorStop::auto(stop.color))
                                    .collect(),
                                position: Position::CENTER,
                            }),
                            BorderGradient::from(LinearGradient {
                                angle: 3. * TAU / 8.,
                                stops: vec![YELLOW.into(), Color::WHITE.into(), ORANGE.into()],
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
            if let Gradient::Linear(LinearGradient { angle, .. }) = gradient {
                *angle += 0.5 * time.delta_secs();
            }
        }
    }
}
