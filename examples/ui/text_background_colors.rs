//! This example demonstrates UI text with a background color

use bevy::{
    color::palettes::css::{BLUE, GREEN, PURPLE, RED, YELLOW},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2d);
    // Text with one section
    commands
        .spawn((
            Text::default(),
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            TextFont {
                // This font is loaded and will be used instead of the default font.
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 67.0,
                ..default()
            },
        ))
        .with_children(|commands| {
            for (text, color) in [
                ("A", RED),
                ("B", GREEN),
                ("C", BLUE),
                ("D", YELLOW),
                ("E", PURPLE),
            ] {
                commands.spawn((TextSpan::new(text), TextColor(color.into())));
            }
        });
}
