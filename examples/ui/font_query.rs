//! This example demonstrates how to use font weights, widths and styles.

use bevy::prelude::*;
use bevy::text::CosmicFontSystem;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut font_system: ResMut<CosmicFontSystem>) {
    font_system.0.db_mut().load_system_fonts();

    let family = Some("Noto Sans".to_string());

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
                    family: family.clone(),
                    font_size: 32.0,
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
                                    family: family.clone(),
                                    weight: FontWeight::THIN,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 200 (Extra Light)"),
                                TextFont {
                                    family: family.clone(),
                                    weight: FontWeight::EXTRA_LIGHT,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 300 (Light)"),
                                TextFont {
                                    family: family.clone(),
                                    weight: FontWeight::LIGHT,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 400 (Normal)"),
                                TextFont {
                                    family: family.clone(),
                                    weight: FontWeight::NORMAL,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 500 (Medium)"),
                                TextFont {
                                    family: family.clone(),
                                    weight: FontWeight::MEDIUM,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 600 (Semibold)"),
                                TextFont {
                                    family: family.clone(),
                                    weight: FontWeight::SEMIBOLD,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 700 (Bold)"),
                                TextFont {
                                    family: family.clone(),
                                    weight: FontWeight::BOLD,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 800 (Extra Bold)"),
                                TextFont {
                                    family: family.clone(),
                                    weight: FontWeight::EXTRA_BOLD,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("Weight 900 (Black)"),
                                TextFont {
                                    family: family.clone(),
                                    weight: FontWeight::BLACK,
                                    ..default()
                                },
                            ),
                        ]
                    ),
                    (
                        // Right column: Widths (wdth axis)
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
                                    family: family.clone(),
                                    width: FontWidth::ULTRA_CONDENSED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::EXTRA_CONDENSED"),
                                TextFont {
                                    family: family.clone(),
                                    width: FontWidth::EXTRA_CONDENSED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::CONDENSED"),
                                TextFont {
                                    family: family.clone(),
                                    width: FontWidth::CONDENSED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::SEMI_CONDENSED"),
                                TextFont {
                                    family: family.clone(),
                                    width: FontWidth::SEMI_CONDENSED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::NORMAL"),
                                TextFont {
                                    family: family.clone(),
                                    width: FontWidth::NORMAL,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::SEMI_EXPANDED"),
                                TextFont {
                                    family: family.clone(),
                                    width: FontWidth::SEMI_EXPANDED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::EXPANDED"),
                                TextFont {
                                    family: family.clone(),
                                    width: FontWidth::EXPANDED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::EXTRA_EXPANDED"),
                                TextFont {
                                    family: family.clone(),
                                    width: FontWidth::EXTRA_EXPANDED,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontWidth::ULTRA_EXPANDED"),
                                TextFont {
                                    family: family.clone(),
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
                                    family: family.clone(),
                                    style: FontStyle::Normal,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontStyle::Oblique"),
                                TextFont {
                                    family: family.clone(),
                                    style: FontStyle::Oblique,
                                    ..default()
                                },
                            ),
                            (
                                Text::new("FontStyle::Italic"),
                                TextFont {
                                    family: family.clone(),
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
