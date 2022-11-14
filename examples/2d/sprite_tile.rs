//! Displays a single [`Sprite`] tiled in a grid

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: asset_server.load("branding/icon.png"),
        sprite: Sprite {
            custom_size: Some(Vec2::splat(512.0)), // The image size is 256px
            draw_mode: SpriteDrawMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: 1.0, // The image will tile every 256px
            },
            ..default()
        },
        ..default()
    });
}
