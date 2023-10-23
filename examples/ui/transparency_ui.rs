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
    commands.spawn(Camera2dBundle::default());

    let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceAround,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(ButtonBundle {
                    style: Style {
                        width: Val::Px(150.0),
                        height: Val::Px(65.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    image: UiImage::default().with_color(Color::srgb(0.1, 0.5, 0.1)),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Button 1",
                        TextStyle {
                            font: font_handle.clone(),
                            font_size: 40.0,
                            // Alpha channel of the color controls transparency.
                            color: Color::srgba(1.0, 1.0, 1.0, 0.2),
                        },
                    ));
                });

            // Button with a different color,
            // to demonstrate the text looks different due to its transparency.
            parent
                .spawn(ButtonBundle {
                    style: Style {
                        width: Val::Px(150.0),
                        height: Val::Px(65.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    image: UiImage::default().with_color(Color::srgb(0.5, 0.1, 0.5)),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Button 2",
                        TextStyle {
                            font: font_handle.clone(),
                            font_size: 40.0,
                            // Alpha channel of the color controls transparency.
                            color: Color::srgba(1.0, 1.0, 1.0, 0.2),
                        },
                    ));
                });
        });
}
