//! Demonstrates how to use transparency with UI.
//! Shows two colored buttons with transparent text.

use bevy::prelude::*;
use bevy::ui_widgets::Button;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let font_handle = FontSource::from(asset_server.load("fonts/FiraSans-Bold.ttf"));

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
                            font_size: FontSize::Px(33.0),
                            ..default()
                        },
                        // Alpha channel of the color controls transparency.
                        // An alpha value of 0.2 means that the color is mostly transparent,
                        // but a little bit of the RGB color is still visible.
                        // (The color is white in this case since RGB values are all 1.0)

                        // An alpha value of 1.0 means that the color is not transparent at all.
                        // An alpha value of 0.0 means the color is completely transparent.
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                    ));
                });

            // Button with a different background color,
            // to demonstrate that the same color text looks different due to its transparency.
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
                            font_size: FontSize::Px(33.0),
                            ..default()
                        },
                        // Alpha channel of the color controls transparency.
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                    ));
                });
        });
}
