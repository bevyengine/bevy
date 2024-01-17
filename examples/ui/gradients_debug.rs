//! Example demonstrating gradients

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle::default())
        .with_children(|commands| {
            commands.spawn((
                NodeBundle {
                    style: Style {
                        margin: UiRect::all(Val::Px(100.)),
                        width: Val::Px(200.),
                        height: Val::Px(200.),
                        ..default()
                    },
                    background_color: RadialGradient {
                        center: RelativePosition::TOP_LEFT,
                        shape: RadialGradientShape::Circle(RadialGradientExtent::ClosestSide),
                        stops: vec![
                            Color::ORANGE_RED.into(),
                            (Color::RED, Val::Percent(30.)).into(),
                            (Color::YELLOW, Val::Percent(60.)).into(),
                            (Color::MAROON, Val::Percent(80.)).into(),
                            (Color::NONE, Val::Percent(81.)).into(),
                        ],
                    }
                    .into(),
                    ..default()
                },
                Outline {
                    width: Val::Px(1.),
                    color: Color::WHITE,
                    ..Default::default()
                },
            ));

            commands.spawn((
                NodeBundle {
                    style: Style {
                        margin: UiRect::all(Val::Px(100.)),
                        width: Val::Px(200.),
                        height: Val::Px(200.),
                        ..default()
                    },
                    background_color: RadialGradient {
                        center: RelativePosition::CENTER,
                        shape: RadialGradientShape::Circle(RadialGradientExtent::ClosestSide),
                        stops: vec![
                            Color::ORANGE_RED.into(),
                            (Color::RED, Val::Percent(30.)).into(),
                            (Color::YELLOW, Val::Percent(60.)).into(),
                            (Color::MAROON, Val::Percent(80.)).into(),
                            (Color::NONE, Val::Percent(81.)).into(),
                        ],
                    }
                    .into(),
                    ..default()
                },
                Outline {
                    width: Val::Px(1.),
                    color: Color::WHITE,
                    ..Default::default()
                },
            ));
        });
}
