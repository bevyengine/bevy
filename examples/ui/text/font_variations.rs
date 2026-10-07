//! This example demonstrates how to use font variations to control variable font axes.

use bevy::prelude::*;
use bevy::text::{FontVariationTag, FontVariations};

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
                Text::new("Font Variations (wght axis)"),
                TextFont {
                    font: font.clone(),
                    font_size: FontSize::Px(32.0),
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
                Children::spawn(SpawnIter(
                    [100, 200, 300, 400, 500, 600, 700, 800, 900]
                        .into_iter()
                        .map(move |weight| (
                            Text(format!("wght {weight}")),
                            TextFont {
                                font: font.clone(),
                                font_size: FontSize::Px(32.0),
                                font_variations: FontVariations::builder()
                                    .set(FontVariationTag::WEIGHT, weight as f32)
                                    .build(),
                                ..default()
                            },
                        ))
                )),
            ),
        ],
    ));
}
