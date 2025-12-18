//! This example demonstrates how to use font weights with text.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/MonaSans-VariableFont.ttf");

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
                        Text::new("Weight 100 (Thin)"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight::THIN, // 100
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 200 (Extra Light)"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight::EXTRA_LIGHT, // 200
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 300 (Light)"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight::LIGHT, // 300
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 400 (Normal)"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight::NORMAL, // 400
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 500 (Medium)"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight::MEDIUM, // 500
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 600 (Semibold)"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight::SEMIBOLD, // 600
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 700 (Bold)"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight::BOLD, // 700
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 800 (Extra Bold)"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight::EXTRA_BOLD, // 800
                            ..default()
                        },
                    ),
                    (
                        Text::new("Weight 900 (Black)"),
                        TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            weight: FontWeight::BLACK, // 900
                            ..default()
                        },
                    ),
                ]
            ),
        ],
    ));
}
