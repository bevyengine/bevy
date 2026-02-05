//! An example demonstrating overlaid gradients

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;
use core::f32::consts::TAU;

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
            display: Display::Grid,
            width: percent(100),
            height: percent(100),

            ..Default::default()
        })
        .with_children(|commands| {
            commands.spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..Default::default()
                },
                BackgroundColor(Color::BLACK),
                BackgroundGradient(vec![
                    LinearGradient::to_top_right(vec![
                        ColorStop::auto(RED),
                        ColorStop::auto(RED.with_alpha(0.)),
                    ])
                    .into(),
                    LinearGradient::to_top_left(vec![
                        ColorStop::auto(BLUE),
                        ColorStop::auto(BLUE.with_alpha(0.)),
                    ])
                    .into(),
                    ConicGradient {
                        start: 0.,
                        position: UiPosition::CENTER,
                        stops: vec![
                            AngularColorStop::auto(YELLOW.with_alpha(0.)),
                            AngularColorStop::auto(YELLOW.with_alpha(0.)),
                            AngularColorStop::auto(YELLOW),
                            AngularColorStop::auto(YELLOW.with_alpha(0.)),
                            AngularColorStop::auto(YELLOW.with_alpha(0.)),
                        ],
                        ..Default::default()
                    }
                    .into(),
                    RadialGradient {
                        position: UiPosition::TOP.at_x(percent(5)),
                        shape: RadialGradientShape::Circle(vh(30)),
                        stops: vec![
                            ColorStop::auto(Color::WHITE),
                            ColorStop::auto(YELLOW),
                            ColorStop::auto(YELLOW.with_alpha(0.1)),
                            ColorStop::auto(YELLOW.with_alpha(0.)),
                        ],
                        ..Default::default()
                    }
                    .into(),
                    LinearGradient {
                        angle: TAU / 16.,
                        stops: vec![
                            ColorStop::auto(Color::BLACK),
                            ColorStop::auto(Color::BLACK.with_alpha(0.)),
                        ],
                        ..Default::default()
                    }
                    .into(),
                    LinearGradient {
                        angle: 15. * TAU / 16.,
                        stops: vec![
                            ColorStop::auto(Color::BLACK),
                            ColorStop::auto(Color::BLACK.with_alpha(0.)),
                        ],
                        ..Default::default()
                    }
                    .into(),
                ]),
            ));
        });
}
