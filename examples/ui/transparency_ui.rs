//! Demonstrates how to use transparency with UI.
//! Shows two colored buttons with transparent text.

use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceAround,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: px(150),
                        height: px(65),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.1, 0.5, 0.1)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Button 1"),
                        TextFont {
                            font: font_handle.clone(),
                            font_size: 33.0,
                            ..default()
                        },
                        // Alpha channel of the color controls transparency.
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                    ));
                });

            // Button with a different color,
            // to demonstrate the text looks different due to its transparency.
            parent
                .spawn((
                    Button,
                    Node {
                        width: px(150),
                        height: px(65),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.5, 0.1, 0.5)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Button 2"),
                        TextFont {
                            font: font_handle.clone(),
                            font_size: 33.0,
                            ..default()
                        },
                        // Alpha channel of the color controls transparency.
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                    ));
                });
        });
}
