//! Displays a single [`Sprite`], created from an image, but flipped on one axis.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());
    commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("branding/icon.png"),
        sprite: Sprite::default(),
        // Use a negative x scaling to flip the logo to the left
        transform: Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0)),
        ..default()
    });
}
