//! Showcase how to use and configure FPS overlay.

use std::any::TypeId;

use bevy::{
    dev_tools::{
        fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
        DevToolsStore,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_config: TextStyle {
                        // Here we define size of our overlay
                        font_size: 50.0,
                        // We can also change color of the overlay
                        color: Color::srgb(0.0, 1.0, 0.0),
                        // If we want, we can use a custom font
                        font: default(),
                    },
                },
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, customize_config)
        .run();
}

fn setup(mut commands: Commands) {
    // We need to spawn camera to see overlay
    commands.spawn(Camera2dBundle::default());
    commands.spawn(
        TextBundle::from_sections([
            TextSection::new(
                "Press 1 to change color of the overlay.",
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

fn customize_config(input: Res<ButtonInput<KeyCode>>, mut dev_tools: ResMut<DevToolsStore>) {
    // We try to get mutable reference to fps overlay dev tool. Otherwise we don't do anything
    let Some(dev_tool) = dev_tools.get_mut(&TypeId::of::<FpsOverlayConfig>()) else {
        return;
    };

    // We try to access configuration struct that is specific to this dev tool.
    let Some(tool_config) = dev_tool.get_tool_config_mut::<FpsOverlayConfig>() else {
        return;
    };

    if input.just_pressed(KeyCode::Digit1) {
        // Changing tool_config will affect overlay
        tool_config.text_config.color = Color::srgb(1.0, 0.0, 0.0);
    }
    if input.just_pressed(KeyCode::Digit2) {
        tool_config.text_config.font_size -= 2.0;
    }
}
