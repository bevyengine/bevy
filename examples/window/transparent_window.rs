//! Shows how to display a window in transparent mode.
//!
//! This feature works as expected depending on the platform. Please check the
//! [documentation](https://docs.rs/bevy/latest/bevy/prelude/struct.Window.html#structfield.transparent)
//! for more details.

#[cfg(target_os = "macos")]
use bevy::window::CompositeAlphaMode;
use bevy::{
    prelude::*,
    window::{Window, WindowPlugin},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // Setting `transparent` allows the `ClearColor`'s alpha value to take effect
                transparent: true,
                // Disabling window decorations to make it feel more like a widget than a window
                decorations: false,
                #[cfg(target_os = "macos")]
                composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                ..default()
            }),
            ..default()
        }))
        // ClearColor must have 0 alpha, otherwise some color will bleed through
        .insert_resource(ClearColor(Color::NONE))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: asset_server.load("branding/icon.png"),
        ..default()
    });
}
