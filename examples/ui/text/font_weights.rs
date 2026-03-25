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
                    [100, 134, 200, 300, 400, 500, 600, 700, 800, 900, 950]
                        .into_iter()
                        .map(move |weight| (
                            Text(format!("Weight {weight}")),
                            TextFont {
                                font: font.clone(),
                                font_size: FontSize::Px(32.0),
                                weight: FontWeight(weight),
                                ..default()
                            },
                        ))
                ))
            ),
        ],
    ));
}
