//! Simple example demonstrating radial gradients.

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
            for (radial_gradient_axis, a) in [
                (RadialGradientAxis::ClosestSide, "closest"),
                (RadialGradientAxis::FarthestSide, "farthest"),
                (RadialGradientAxis::Length(Val::Percent(110.)), "110%"),
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
                                        GradientNode(Gradient::Radial {
                                            stops: color_stops.clone(),
                                            center: RelativePosition::new(x, y),
                                            shape: RadialGradientShape::Circle(
                                                radial_gradient_axis,
                                            ),
                                        }),
                                    ));
                                });
                        }
                    }
                }
            }

            for (radial_gradient_axis_x, radial_gradient_axis_y, a) in [
                (
                    RadialGradientAxis::Length(Val::Px(50.)),
                    RadialGradientAxis::Length(Val::Px(20.)),
                    "50x20",
                ),
                (
                    RadialGradientAxis::ClosestSide,
                    RadialGradientAxis::FarthestSide,
                    "close, far",
                ),
                (
                    RadialGradientAxis::FarthestSide,
                    RadialGradientAxis::ClosestSide,
                    "far, close",
                ),
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
                                        GradientNode(Gradient::Radial {
                                            stops: color_stops.clone(),
                                            center: RelativePosition::new(x, y),
                                            shape: RadialGradientShape::Ellipse(
                                                radial_gradient_axis_x,
                                                radial_gradient_axis_y,
                                            ),
                                        }),
                                    ));
                                });
                        }
                    }
                }
            }
        });
}
