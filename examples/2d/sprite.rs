//! Shows how to render simple rectangle sprite with a single color.
//!
//! Since this [`Sprite`] is not generated from an image, we have create it directly when spawning
//! the [`SpriteBundle`] and specify the size using `custom_size`.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("branding/icon.png"),
        ..default()
    });
}
