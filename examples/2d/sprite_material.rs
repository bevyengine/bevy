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

    let mut sprite = SpriteMesh::from_image(texture_handle.clone());
    sprite.alpha_mode = SpriteAlphaMode::Blend;

    sprite.rect = Some(Rect {
        min: vec2(50.0, 100.0),
        max: vec2(150.0, 200.0),
    });

    commands.spawn((sprite, Transform::from_translation(vec3(-100.0, 0.0, 0.0))));

    let mut sprite = Sprite::from_image(texture_handle.clone());

    sprite.rect = Some(Rect {
        min: vec2(50.0, 100.0),
        max: vec2(150.0, 200.0),
    });

    commands.spawn((sprite, Transform::from_translation(vec3(100.0, 0.0, 0.0))));
}
