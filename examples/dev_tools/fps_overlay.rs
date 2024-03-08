//! Showcase how to use and configure FPS overlay.

use bevy::{
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    // Here we define size of our overlay
                    font_size: 50.0,
                    // We can also change color of the overlay
                    font_color: Color::srgb(0.0, 1.0, 0.0),
                    // If we want, we can use a custom font
                    font_path: None,
                    // This keybind will be toggling on/off the overlay
                    keybind: KeyCode::Escape,
                },
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, system)
        .run();
}

fn setup(mut commands: Commands) {
    // We need to spawn camera to see overlay
    commands.spawn(Camera2dBundle::default());
    commands.spawn(
        TextBundle::from_sections([
            TextSection::new(
                "Press ESC to toggle overlay",
                TextStyle {
                    font_size: 25.0,
                    ..default()
                },
            ),
            TextSection::new(
                "\nPress 1 to change color of the overlay.",
                TextStyle {
                    font_size: 25.0,
                    ..default()
                },
            ),
            TextSection::new(
                "\nPress 2 to change size of the overlay",
                TextStyle {
                    font_size: 25.0,
                    ..default()
                },
            ),
        ])
        .with_style(Style {
            justify_self: JustifySelf::Center,
            ..default()
        }),
    );
}

fn system(input: Res<ButtonInput<KeyCode>>, mut overlay: ResMut<FpsOverlayConfig>) {
    if input.just_pressed(KeyCode::Digit1) {
        // Changing resource will affect overlay
        overlay.font_color = Color::srgb(1.0, 0.0, 0.0);
    }
    if input.just_pressed(KeyCode::Digit2) {
        overlay.font_size -= 2.0;
    }
}
