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

    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..Default::default()
        },
        BackgroundColor(Color::BLACK),
        BackgroundGradient(vec![
            Gradient::linear_to_top_right(vec![
                ColorStop::auto(RED.into()),
                ColorStop::auto(RED.with_alpha(0.).into()),
            ]),
            Gradient::linear_to_top_left(vec![
                ColorStop::auto(BLUE.into()),
                ColorStop::auto(BLUE.with_alpha(0.).into()),
            ]),
            Gradient::Conic {
                position: RelativePosition::CENTER,
                stops: vec![
                    AngularColorStop::auto(YELLOW.with_alpha(0.).into()),
                    AngularColorStop::auto(YELLOW.with_alpha(0.).into()),
                    AngularColorStop::auto(YELLOW.into()),
                    AngularColorStop::auto(YELLOW.with_alpha(0.).into()),
                    AngularColorStop::auto(YELLOW.with_alpha(0.).into()),
                ],
            },
            Gradient::Radial {
                position: RelativePosition::top(Val::Percent(5.)),
                shape: RadialGradientShape::Circle(Val::Vh(30.)),
                stops: vec![
                    ColorStop::auto(Color::WHITE),
                    ColorStop::auto(YELLOW.into()),
                    ColorStop::auto(YELLOW.with_alpha(0.1).into()),
                    ColorStop::auto(YELLOW.with_alpha(0.).into()),
                ],
            },
            Gradient::Linear {
                angle: TAU / 16.,
                stops: vec![
                    ColorStop::auto(Color::BLACK),
                    ColorStop::auto(Color::BLACK.with_alpha(0.)),
                ],
            },
            Gradient::Linear {
                angle: 15. * TAU / 16.,
                stops: vec![
                    ColorStop::auto(Color::BLACK),
                    ColorStop::auto(Color::BLACK.with_alpha(0.)),
                ],
            },
        ]),
    ));
}
