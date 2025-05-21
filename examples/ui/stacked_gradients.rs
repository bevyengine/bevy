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
            width: Val::Percent(100.),
            height: Val::Percent(100.),

            ..Default::default()
        })
        .with_children(|commands| {
            commands.spawn((
                Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
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
                        position: Position::CENTER,
                        stops: vec![
                            AngularColorStop::auto(YELLOW.with_alpha(0.)),
                            AngularColorStop::auto(YELLOW.with_alpha(0.)),
                            AngularColorStop::auto(YELLOW),
                            AngularColorStop::auto(YELLOW.with_alpha(0.)),
                            AngularColorStop::auto(YELLOW.with_alpha(0.)),
                        ],
                    }
                    .into(),
                    RadialGradient {
                        position: Position::TOP.at_x(Val::Percent(5.)),
                        shape: RadialGradientShape::Circle(Val::Vh(30.)),
                        stops: vec![
                            ColorStop::auto(Color::WHITE),
                            ColorStop::auto(YELLOW),
                            ColorStop::auto(YELLOW.with_alpha(0.1)),
                            ColorStop::auto(YELLOW.with_alpha(0.)),
                        ],
                    }
                    .into(),
                    LinearGradient {
                        angle: TAU / 16.,
                        stops: vec![
                            ColorStop::auto(Color::BLACK),
                            ColorStop::auto(Color::BLACK.with_alpha(0.)),
                        ],
                    }
                    .into(),
                    LinearGradient {
                        angle: 15. * TAU / 16.,
                        stops: vec![
                            ColorStop::auto(Color::BLACK),
                            ColorStop::auto(Color::BLACK.with_alpha(0.)),
                        ],
                    }
                    .into(),
                ]),
            ));
        });
}
