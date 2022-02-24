/// An example of how to display a window in transparent mode
/// [Documentation & Platform support.](https://docs.rs/bevy/latest/bevy/prelude/struct.WindowDescriptor.html#structfield.transparent)
use bevy::{prelude::*, window::WindowDescriptor};

fn main() {
    App::new()
        // ClearColor must have 0 alpha, otherwise some color will bleed through
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(WindowDescriptor {
            // Setting `transparent` allows the `ClearColor`'s alpha value to take effect
            transparent: true,
            // Disabling window decorations to make it feel more like a widget than a window
            decorations: false,
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_plugins(DefaultPlugins)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("branding/icon.png"),
        ..Default::default()
    });
}
