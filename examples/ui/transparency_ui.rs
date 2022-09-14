//! Demonstrates how to use transparency with UI.
//! Shows two colored buttons with transparent text.

use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());

    let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                margin: UiRect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            color: Color::rgb(0.1, 0.5, 0.1).into(),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle::from_section(
                "Button 1",
                TextStyle {
                    font: font_handle.clone(),
                    font_size: 40.0,
                    // Alpha channel of the color controls transparency.
                    color: Color::rgba(1.0, 1.0, 1.0, 0.2),
                },
            ));
        });

    // Button with a different color,
    // to demonstrate the text looks different due to its transparency.
    commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                margin: UiRect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            color: Color::rgb(0.5, 0.1, 0.5).into(),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle::from_section(
                "Button 2",
                TextStyle {
                    font: font_handle.clone(),
                    font_size: 40.0,
                    // Alpha channel of the color controls transparency.
                    color: Color::rgba(1.0, 1.0, 1.0, 0.2),
                },
            ));
        });
}
