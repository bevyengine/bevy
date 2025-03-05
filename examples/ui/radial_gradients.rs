//! Simple example demonstrating radial gradients.

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::ORANGE;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;
use bevy::ui::ColorStop;
use std::f32::consts::TAU;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let color_stops = vec![
        ColorStop::new(Color::WHITE, Val::Auto),
        ColorStop::new(Color::WHITE, Val::Percent(100.)),
        ColorStop::new(Color::BLACK, Val::Auto),
    ];

    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            row_gap: Val::Px(30.),
            margin: UiRect::all(Val::Px(30.)),
            ..Default::default()
        })
        .with_children(|commands| {
            commands.spawn(Node::default()).with_children(|commands| {
                commands
                    .spawn((
                        Node {
                            flex_wrap: FlexWrap::Wrap,
                            row_gap: Val::Px(10.),
                            column_gap: Val::Px(10.),
                            ..Default::default()
                        },
                        BackgroundColor(BLUE.into()),
                    ))
                    .with_children(|commands| {
                        for radial_gradient_axis in [
                            RadialGradientAxis::ClosestSide,
                            RadialGradientAxis::FarthestSide,
                        ] {
                            for (w, h) in [(100., 100.), (50., 100.), (100., 50.)] {
                                commands.spawn((
                                    Node {
                                        width: Val::Px(w),
                                        height: Val::Px(h),
                                        ..default()
                                    },
                                    GradientNode(Gradient::Radial {
                                        stops: color_stops.clone(),
                                        center: [
                                            RelativePosition::Center(Val::Auto),
                                            RelativePosition::Center(Val::Auto),
                                        ],
                                        shape: RadialGradientShape::Circle(radial_gradient_axis),
                                    }),
                                ));
                            }
                        }
                    });
            });
        });
}
