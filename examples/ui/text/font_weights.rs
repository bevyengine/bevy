//! This example demonstrates how to use font weights with text.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: FontSource = asset_server.load("fonts/MonaSans-VariableFont.ttf").into();

    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Text::new("Font Weights"),
                TextFont {
                    font: font.clone(),
                    font_size: 32.0,
                    ..default()
                },
                Underline,
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: px(8.).all(),
                    row_gap: px(8.),
                    ..default()
                },
                children![
                    (
                        Text::new("Weight 100"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(100),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 134"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(134),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 200"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(200),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 300"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(300),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 400"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(400),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 500"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(500),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 600"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(600),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 700"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(700),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 800"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(800),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 900"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(900),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 950"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight(950),
                            ..default()
                        },
                    )
                ]
            ),
        ],
    ));
}
