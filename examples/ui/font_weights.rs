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
                            weight: 100.into(),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 134"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: 134.into(),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 200"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: 200.into(),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 300"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: 300.into(),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 400"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: 400.into(),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 500"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: 500.into(),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 600"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: 600.into(),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 700"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: 700.into(),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 800"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: 800.into(),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 900"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: 900.into(),
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 950"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: 950.into(),
                            ..default()
                        },
                    )
                ]
            ),
        ],
    ));
}
