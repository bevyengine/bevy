//! This example demonstrates the usage of '.meta' files to override the default settings for loading an asset

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SpriteBundle {
        texture: asset_server.load("pixel/bevy_pixel_dark.png"),
        sprite: Sprite { 
            custom_size: Some(Vec2 { x: 160.0, y: 120.0 }), 
            ..Default::default() 
        },
        transform: Transform::from_xyz(-100.0, 0.0, 0.0),
        ..Default::default()
    });
    commands.spawn(SpriteBundle {
        texture: asset_server.load("pixel/bevy_pixel_dark.png"),
        sprite: Sprite { 
            custom_size: Some(Vec2 { x: 160.0, y: 120.0 }), 
            ..Default::default() 
        },
        transform: Transform::from_xyz(100.0, 0.0, 0.0),
        ..Default::default()
    });
    commands.spawn(Camera2dBundle::default());
}