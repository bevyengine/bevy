//! Simple example demonstrating radial gradients.

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::RED;
use bevy::prelude::*;
use bevy::ui::ColorStop;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_grid)
        .run();
}

const CELL_SIZE: f32 = 100.;
const GAP: f32 = 10.;

fn setup_grid(mut commands: Commands) {
    let color_stops = vec![
        ColorStop::new(Color::BLACK, Val::Px(5.)),
        ColorStop::new(Color::WHITE, Val::Px(5.)),
        ColorStop::new(Color::WHITE, Val::Percent(100.)),
        ColorStop::new(RED.into(), Val::Auto),
    ];

    let stops_2 = vec![ColorStop::auto(RED.into()), ColorStop::auto(BLUE.into())];

    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                display: Display::Grid,
                align_items: AlignItems::Start,
                align_content: AlignContent::Start,
                grid_template_columns: vec![RepeatedGridTrack::px(
                    GridTrackRepetition::AutoFill,
                    CELL_SIZE,
                )],
                grid_auto_flow: GridAutoFlow::Row,
                row_gap: Val::Px(GAP),
                column_gap: Val::Px(GAP),
                margin: UiRect::all(Val::Px(GAP)),
                ..Default::default()
            },
            BackgroundColor(NAVY.into()),
        ))
        .with_children(|commands| {
            commands
                .spawn((
                    BackgroundColor(GREEN.into()),
                    Node {
                        display: Display::Grid,
                        width: Val::Px(CELL_SIZE),
                        ..Default::default()
                    },
                ))
                .with_children(|commands| {
                    commands.spawn((Text(format!("-\n-")), TextFont::from_font_size(10.)));
                    commands.spawn((
                        Node {
                            width: Val::Px(100.),
                            height: Val::Px(100.),
                            ..default()
                        },
                        BackgroundGradient::from(Gradient::Radial {
                            stops: vec![
                                ColorStop::new(RED.into(), Val::Auto),
                                ColorStop::new(BLUE.into(), Val::Auto),
                            ],
                            position: RelativePosition::center(Val::ZERO, Val::ZERO),
                            shape: RadialGradientShape::ClosestSide,
                        }),
                    ));
                });

            commands
                .spawn((
                    BackgroundColor(GREEN.into()),
                    Node {
                        display: Display::Grid,
                        width: Val::Px(CELL_SIZE),
                        ..Default::default()
                    },
                ))
                .with_children(|commands| {
                    commands.spawn((Text(format!("-\n-")), TextFont::from_font_size(10.)));
                    commands.spawn((
                        Node {
                            width: Val::Px(100.),
                            height: Val::Px(100.),
                            ..default()
                        },
                        BackgroundGradient::from(Gradient::Radial {
                            stops: vec![
                                ColorStop::new(RED.into(), Val::Auto),
                                ColorStop::new(BLUE.into(), Val::Auto),
                            ],
                            position: RelativePosition::center(Val::ZERO, Val::ZERO),
                            shape: RadialGradientShape::Circle(Val::Px(50.)),
                        }),
                    ));
                });

            commands
                .spawn((
                    BackgroundColor(GREEN.into()),
                    Node {
                        display: Display::Grid,
                        width: Val::Px(CELL_SIZE),
                        ..Default::default()
                    },
                ))
                .with_children(|commands| {
                    commands.spawn((Text(format!("-\n-")), TextFont::from_font_size(10.)));
                    commands.spawn((
                        Node {
                            width: Val::Px(100.),
                            height: Val::Px(100.),
                            ..default()
                        },
                        BackgroundGradient::from(Gradient::Radial {
                            stops: vec![
                                ColorStop::new(RED.into(), Val::Auto),
                                ColorStop::new(BLUE.into(), Val::Auto),
                            ],
                            position: RelativePosition::center(Val::ZERO, Val::ZERO),
                            shape: RadialGradientShape::Ellipse(Val::Px(50.), Val::Px(25.)),
                        }),
                    ));
                });
            commands
                .spawn((
                    BackgroundColor(GREEN.into()),
                    Node {
                        display: Display::Grid,
                        width: Val::Px(CELL_SIZE),
                        ..Default::default()
                    },
                ))
                .with_children(|commands| {
                    commands.spawn((Text(format!("-\n-")), TextFont::from_font_size(10.)));
                    commands.spawn((
                        Node {
                            width: Val::Px(100.),
                            height: Val::Px(100.),
                            ..default()
                        },
                        BackgroundGradient::from(Gradient::Radial {
                            stops: vec![
                                ColorStop::new(RED.into(), Val::Auto),
                                ColorStop::new(BLUE.into(), Val::Auto),
                            ],
                            position: RelativePosition::top_right(Val::ZERO, Val::ZERO),
                            shape: RadialGradientShape::Ellipse(Val::Px(50.), Val::Px(25.)),
                        }),
                    ));
                });

            commands
                .spawn((
                    BackgroundColor(GREEN.into()),
                    Node {
                        display: Display::Grid,
                        width: Val::Px(CELL_SIZE),
                        ..Default::default()
                    },
                ))
                .with_children(|commands| {
                    commands.spawn((Text(format!("-\n-")), TextFont::from_font_size(10.)));
                    commands.spawn((
                        Node {
                            width: Val::Px(100.),
                            height: Val::Px(100.),
                            ..default()
                        },
                        BackgroundGradient::from(Gradient::Radial {
                            stops: vec![
                                ColorStop::new(RED.into(), Val::Auto),
                                ColorStop::new(BLUE.into(), Val::Auto),
                            ],
                            position: RelativePosition::bottom_right(Val::ZERO, Val::ZERO),
                            shape: RadialGradientShape::Ellipse(Val::Px(50.), Val::Px(25.)),
                        }),
                    ));
                });

            commands
                .spawn((
                    BackgroundColor(GREEN.into()),
                    Node {
                        display: Display::Grid,
                        width: Val::Px(CELL_SIZE),
                        ..Default::default()
                    },
                ))
                .with_children(|commands| {
                    commands.spawn((Text(format!("-\n-")), TextFont::from_font_size(10.)));
                    commands.spawn((
                        Node {
                            width: Val::Px(100.),
                            height: Val::Px(100.),
                            ..default()
                        },
                        BackgroundGradient::from(Gradient::Radial {
                            stops: vec![
                                ColorStop::new(RED.into(), Val::Auto),
                                ColorStop::new(BLUE.into(), Val::Auto),
                            ],
                            position: RelativePosition::left(Val::ZERO),
                            shape: RadialGradientShape::Ellipse(Val::Px(50.), Val::Px(25.)),
                        }),
                    ));
                });

            commands
                .spawn((
                    BackgroundColor(GREEN.into()),
                    Node {
                        display: Display::Grid,
                        width: Val::Px(CELL_SIZE),
                        ..Default::default()
                    },
                ))
                .with_children(|commands| {
                    commands.spawn((Text(format!("-\n-")), TextFont::from_font_size(10.)));
                    commands.spawn((
                        Node {
                            width: Val::Px(100.),
                            height: Val::Px(100.),
                            ..default()
                        },
                        BackgroundGradient::from(Gradient::Radial {
                            stops: vec![
                                ColorStop::new(RED.into(), Val::Auto),
                                ColorStop::new(BLUE.into(), Val::Auto),
                            ],
                            position: RelativePosition::top(Val::ZERO),
                            shape: RadialGradientShape::Ellipse(Val::Px(50.), Val::Px(25.)),
                        }),
                    ));
                });

            for (radial_gradient_axis, a) in [
                (RadialGradientShape::ClosestSide, "closest"),
                (RadialGradientShape::FarthestSide, "farthest"),
                (RadialGradientShape::Circle(Val::Percent(55.)), "55%"),
            ] {
                for (x, b) in [
                    (RelativeVal::Start(Val::ZERO), "start0"),
                    (RelativeVal::Start(Val::Percent(20.)), "start"),
                    (RelativeVal::center(), "center"),
                    (RelativeVal::End(Val::Percent(20.)), "end"),
                ] {
                    for (y, c) in [
                        (RelativeVal::Start(Val::Percent(20.)), "start"),
                        (RelativeVal::center(), "center"),
                        (RelativeVal::End(Val::Percent(20.)), "end"),
                    ] {
                        for (w, h) in [(100., 100.)] {
                            //, (50., 100.), (100., 50.)] {

                            for stops in [color_stops.clone(), stops_2.clone()] {
                                commands
                                    .spawn((
                                        BackgroundColor(GREEN.into()),
                                        Node {
                                            display: Display::Grid,
                                            width: Val::Px(CELL_SIZE),
                                            ..Default::default()
                                        },
                                    ))
                                    .with_children(|commands| {
                                        commands.spawn((
                                            Text(format!("{a}\n{b}, {c}")),
                                            TextFont::from_font_size(10.),
                                        ));
                                        commands.spawn((
                                            Node {
                                                width: Val::Px(w),
                                                height: Val::Px(h),
                                                ..default()
                                            },
                                            BackgroundGradient::from(Gradient::Radial {
                                                stops,
                                                position: RelativePosition::new(x, y),
                                                shape: radial_gradient_axis,
                                            }),
                                        ));
                                    });
                            }
                        }
                    }
                }
            }

            // for (radial_gradient_axis_x, radial_gradient_axis_y, a) in [
            //     (
            //         RadialGradie
            //         "50x20",
            //     ),
            //     (Spread::ClosestSide, Spread::FarthestSide, "close, far"),
            //     (Spread::FarthestSide, Spread::ClosestSide, "far, close"),
            // ] {
            //     for (x, b) in [
            //         (RelativeVal::Start(Val::ZERO), "start0"),
            //         (RelativeVal::Start(Val::Percent(20.)), "start"),
            //         (RelativeVal::center(), "center"),
            //         (RelativeVal::End(Val::Percent(20.)), "end"),
            //     ] {
            //         for (y, c) in [
            //             (RelativeVal::Start(Val::Percent(20.)), "start"),
            //             (RelativeVal::center(), "center"),
            //             (RelativeVal::End(Val::Percent(20.)), "end"),
            //         ] {
            //             for (w, h) in [(100., 100.)] {
            //                 //, (50., 100.), (100., 50.)] {

            //                 commands
            //                     .spawn((
            //                         BackgroundColor(GREEN.into()),
            //                         Node {
            //                             display: Display::Grid,
            //                             width: Val::Px(CELL_SIZE),
            //                             ..Default::default()
            //                         },
            //                     ))
            //                     .with_children(|commands| {
            //                         commands.spawn((
            //                             Text(format!("{a}\n{b}, {c}")),
            //                             TextFont::from_font_size(10.),
            //                         ));
            //                         commands.spawn((
            //                             Node {
            //                                 width: Val::Px(w),
            //                                 height: Val::Px(h),
            //                                 ..default()
            //                             },
            //                             GradientNode(Gradient::Radial {
            //                                 stops: color_stops.clone(),
            //                                 center: RelativePosition::new(x, y),
            //                                 shape: RadialGradientShape::Ellipse(
            //                                     radial_gradient_axis_x,
            //                                     radial_gradient_axis_y,
            //                                 ),
            //                             }),
            //                         ));
            //                     });
            //             }
            //         }
            //     }
            // }
        });
}
