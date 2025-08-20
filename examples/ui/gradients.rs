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

#[derive(Component)]
struct CurrentColorSpaceLabel;

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
            row_gap: Val::Px(20.),
            margin: UiRect::all(Val::Px(20.)),
            ..Default::default()
        })
        .with_children(|commands| {
            for (b, stops) in [
                (
                    4.,
                    vec![
                        ColorStop::new(Color::WHITE, Val::Percent(15.)),
                        ColorStop::new(Color::BLACK, Val::Percent(85.)),
                    ],
                ),
                (4., vec![RED.into(), BLUE.into(), LIME.into()]),
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
                            row_gap: Val::Px(5.),
                            ..Default::default()
                        })
                        .with_children(|commands| {
                            for (w, h) in [(70., 70.), (35., 70.), (70., 35.)] {
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
                                                    ..default()
                                                }),
                                                BorderGradient::from(LinearGradient {
                                                    angle: 3. * TAU / 8.,
                                                    stops: vec![
                                                        YELLOW.into(),
                                                        Color::WHITE.into(),
                                                        ORANGE.into(),
                                                    ],
                                                    ..default()
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
                                margin: UiRect::left(Val::Px(20.)),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(20.)),
                            BackgroundGradient::from(LinearGradient {
                                angle: 0.,
                                stops: stops.clone(),
                                ..default()
                            }),
                            BorderGradient::from(LinearGradient {
                                angle: 3. * TAU / 8.,
                                stops: vec![YELLOW.into(), Color::WHITE.into(), ORANGE.into()],
                                ..default()
                            }),
                            AnimateMarker,
                        ));

                        commands.spawn((
                            Node {
                                aspect_ratio: Some(1.),
                                height: Val::Percent(100.),
                                border: UiRect::all(Val::Px(b)),
                                margin: UiRect::left(Val::Px(20.)),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(20.)),
                            BackgroundGradient::from(RadialGradient {
                                stops: stops.clone(),
                                shape: RadialGradientShape::ClosestSide,
                                position: UiPosition::CENTER,
                                ..default()
                            }),
                            BorderGradient::from(LinearGradient {
                                angle: 3. * TAU / 8.,
                                stops: vec![YELLOW.into(), Color::WHITE.into(), ORANGE.into()],
                                ..default()
                            }),
                            AnimateMarker,
                        ));
                        commands.spawn((
                            Node {
                                aspect_ratio: Some(1.),
                                height: Val::Percent(100.),
                                border: UiRect::all(Val::Px(b)),
                                margin: UiRect::left(Val::Px(20.)),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(20.)),
                            BackgroundGradient::from(ConicGradient {
                                start: 0.,
                                stops: stops
                                    .iter()
                                    .map(|stop| AngularColorStop::auto(stop.color))
                                    .collect(),
                                position: UiPosition::CENTER,
                                ..default()
                            }),
                            BorderGradient::from(LinearGradient {
                                angle: 3. * TAU / 8.,
                                stops: vec![YELLOW.into(), Color::WHITE.into(), ORANGE.into()],
                                ..default()
                            }),
                            AnimateMarker,
                        ));
                    });
                });
            }

            let button = commands.spawn((
                        Button,
                        Node {
                            border: UiRect::all(Val::Px(2.0)),
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                            // horizontally center child text
                            justify_content: JustifyContent::Center,
                            // vertically center child text
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BorderColor::all(Color::WHITE),
                        BorderRadius::MAX,
                        BackgroundColor(Color::BLACK),
                        children![(
                            Text::new("next color space"),
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            TextShadow::default(),
                        )]
                )).observe(
                    |_trigger: On<Pointer<Over>>, mut border_query: Query<&mut BorderColor, With<Button>>| {
                    *border_query.single_mut().unwrap() = BorderColor::all(RED);


                })
                .observe(
                    |_trigger: On<Pointer<Out>>, mut border_query: Query<&mut BorderColor, With<Button>>| {
                    *border_query.single_mut().unwrap() = BorderColor::all(Color::WHITE);
                })
                .observe(
                        |_trigger: On<Pointer<Click>>,
                            mut gradients_query: Query<&mut BackgroundGradient>,
                            mut label_query: Query<
                            &mut Text,
                            With<CurrentColorSpaceLabel>,
                        >| {
                            let mut current_space = InterpolationColorSpace::default();
                            for mut gradients in gradients_query.iter_mut() {
                                for gradient in gradients.0.iter_mut() {
                                    let space = match gradient {
                                        Gradient::Linear(linear_gradient) => {
                                            &mut linear_gradient.color_space
                                        }
                                        Gradient::Radial(radial_gradient) => {
                                            &mut radial_gradient.color_space
                                        }
                                        Gradient::Conic(conic_gradient) => {
                                            &mut conic_gradient.color_space
                                        }
                                    };
                                    *space = match *space {
                                        InterpolationColorSpace::Oklaba => {
                                            InterpolationColorSpace::Oklcha
                                        }
                                        InterpolationColorSpace::Oklcha => {
                                            InterpolationColorSpace::OklchaLong
                                        }
                                        InterpolationColorSpace::OklchaLong => {
                                            InterpolationColorSpace::Srgba
                                        }
                                        InterpolationColorSpace::Srgba => {
                                            InterpolationColorSpace::LinearRgba
                                        }
                                        InterpolationColorSpace::LinearRgba => {
                                            InterpolationColorSpace::Hsla
                                        }
                                        InterpolationColorSpace::Hsla => {
                                            InterpolationColorSpace::HslaLong
                                        }
                                        InterpolationColorSpace::HslaLong => {
                                            InterpolationColorSpace::Hsva
                                        }
                                        InterpolationColorSpace::Hsva => {
                                            InterpolationColorSpace::HsvaLong
                                        }
                                        InterpolationColorSpace::HsvaLong => {
                                            InterpolationColorSpace::Oklaba
                                        }
                                    };
                                    current_space = *space;
                                }
                            }
                            for mut label in label_query.iter_mut() {
                                label.0 = format!("{current_space:?}");
                            }
                        }
                    ).id();

            commands.spawn(
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.),
                    align_items: AlignItems::Center,
                    ..Default::default()
                }
            ).with_children(|commands| {
                commands.spawn((Text::new(format!("{:?}", InterpolationColorSpace::default())), TextFont { font_size: 25., ..default() }, CurrentColorSpaceLabel));

            })
            .add_child(button);
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
