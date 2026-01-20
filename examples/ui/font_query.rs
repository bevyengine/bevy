//! This example demonstrates how to use font weights, widths and styles.

use bevy::prelude::*;

use bevy::text::FontSource;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let family = FontSource::from(asset_server.load("fonts/MonaSans-VariableFont.ttf"));

    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            align_items: AlignItems::Center,
            padding: px(16.).all(),
            row_gap: px(16.),
            ..default()
        },
        children![
            (
                Text::new("Font Weights, Widths & Styles"),
                TextFont {
                    font: family.clone(),
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                Underline,
            ),
            (
                // Two columns side-by-side
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(32.),
                    ..default()
                },
                children![
                    (
                        // Left column: Weights
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
                                    font: family.clone(),
                                    weight: FontWeight::THIN,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 200 (Extra Light)"),
                                TextFont {
                                    font: family.clone(),
                                    weight: FontWeight::EXTRA_LIGHT,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 300 (Light)"),
                                TextFont {
                                    font: family.clone(),
                                    weight: FontWeight::LIGHT,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 400 (Normal)"),
                                TextFont {
                                    font: family.clone(),
                                    weight: FontWeight::NORMAL,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 500 (Medium)"),
                                TextFont {
                                    font: family.clone(),
                                    weight: FontWeight::MEDIUM,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 600 (Semibold)"),
                                TextFont {
                                    font: family.clone(),
                                    weight: FontWeight::SEMIBOLD,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 700 (Bold)"),
                                TextFont {
                                    font: family.clone(),
                                    weight: FontWeight::BOLD,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 800 (Extra Bold)"),
                                TextFont {
                                    font: family.clone(),
                                    weight: FontWeight::EXTRA_BOLD,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 900 (Black)"),
                                TextFont {
                                    font: family.clone(),
                                    weight: FontWeight::BLACK,
                                    ..default()
                                },
                            ),
                        ]
                    ),
                    (
                        // Right column: Widths
                        Node {
                            flex_direction: FlexDirection::Column,
                            padding: px(8.).all(),
                            row_gap: px(8.),
                            ..default()
                        },
                        children![
                            (
                                Text::new("FontWidth::ULTRA_CONDENSED"),
                                TextFont {
                                    font: family.clone(),
                                    width: FontWidth::ULTRA_CONDENSED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::EXTRA_CONDENSED"),
                                TextFont {
                                    font: family.clone(),
                                    width: FontWidth::EXTRA_CONDENSED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::CONDENSED"),
                                TextFont {
                                    font: family.clone(),
                                    width: FontWidth::CONDENSED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::SEMI_CONDENSED"),
                                TextFont {
                                    font: family.clone(),
                                    width: FontWidth::SEMI_CONDENSED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::NORMAL"),
                                TextFont {
                                    font: family.clone(),
                                    width: FontWidth::NORMAL,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::SEMI_EXPANDED"),
                                TextFont {
                                    font: family.clone(),
                                    width: FontWidth::SEMI_EXPANDED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::EXPANDED"),
                                TextFont {
                                    font: family.clone(),
                                    width: FontWidth::EXPANDED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::EXTRA_EXPANDED"),
                                TextFont {
                                    font: family.clone(),
                                    width: FontWidth::EXTRA_EXPANDED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::ULTRA_EXPANDED"),
                                TextFont {
                                    font: family.clone(),
                                    width: FontWidth::ULTRA_EXPANDED,
                                    ..default()
                                },
                            ),
                        ],
                    ),
                    (
                        // Right column: Style
                        Node {
                            flex_direction: FlexDirection::Column,
                            padding: px(8.).all(),
                            row_gap: px(8.),
                            ..default()
                        },
                        children![
                            (
                                Text::new("FontStyle::Normal"),
                                TextFont {
                                    font: family.clone(),
                                    style: FontStyle::Normal,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontStyle::Oblique"),
                                TextFont {
                                    font: family.clone(),
                                    style: FontStyle::Oblique,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontStyle::Italic"),
                                TextFont {
                                    font: family.clone(),
                                    style: FontStyle::Italic,
                                    ..default()
                                },
                            ),
                        ]
                    ),
                ]
            ),
        ],
    ));
}
