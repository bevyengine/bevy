//! Shows how to display a window in transparent mode.
//!
//! This feature works as expected depending on the platform. Please check the
//! [documentation](https://docs.rs/bevy/latest/bevy/prelude/struct.Window.html#structfield.transparent)
//! for more details.

#[cfg(any(target_os = "macos", target_os = "linux"))]
use bevy::window::CompositeAlphaMode;
use bevy::{prelude::*, window::PrimaryWindow};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // ClearColor must have 0 alpha, otherwise some color will bleed through
        .insert_resource(ClearColor(Color::NONE))
        .add_observer(configure_window)
        .add_systems(Startup, setup)
        .run();
}

fn configure_window(trigger: On<Add, PrimaryWindow>, mut window: Query<&mut Window>) {
    let mut window = window.get_mut(trigger.target()).unwrap();

    // Setting `transparent` allows the `ClearColor`'s alpha value to take effect
    window.transparent = true;
    // Disabling window decorations to make it feel more like a widget than a window
    window.decorations = false;

    #[cfg(target_os = "macos")]
    {
        window.composite_alpha_mode = CompositeAlphaMode::PostMultiplied;
    }
    #[cfg(target_os = "linux")]
    {
        window.composite_alpha_mode = CompositeAlphaMode::PreMultiplied;
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(Sprite::from_image(asset_server.load("branding/icon.png")));
}
