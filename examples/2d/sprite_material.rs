//! Displays a single [`Sprite`], created from an image.

use bevy::{
    color::palettes::css::{BLUE, GREEN, WHITE},
    math::Affine2,
    prelude::*,
    sprite::{SpriteAlphaMode, SpriteMesh},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SpriteMaterial>>,
) {
    commands.spawn(Camera2d);

    let texture_handle = asset_server.load("branding/bevy_bird_dark.png");

    commands.spawn(SpriteMesh {
        image: texture_handle,
        texture_atlas: None,
        color: Color::WHITE,
        alpha_mode: SpriteAlphaMode::Mask(0.5),
    });
}
