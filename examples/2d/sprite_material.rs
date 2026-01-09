//! Displays a single [`Sprite`], created from an image.

use bevy::{
    color::palettes::basic::{BLUE, GREEN, PURPLE, RED, WHITE},
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
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut materials: ResMut<Assets<SpriteMaterial>>,
) {
    commands.spawn(Camera2d);

    let texture_handle = asset_server.load("branding/bevy_bird_dark.png");

    let mut sprite = SpriteMesh::from_image(texture_handle.clone());
    sprite.alpha_mode = SpriteAlphaMode::Blend;

    sprite.rect = Some(Rect {
        min: vec2(50.0, 55.0),
        max: vec2(150.0, 400.0),
    });

    sprite.color = Color::Srgba(RED);

    sprite.custom_size = Some(vec2(300.0, 200.0));
    sprite.image_mode = SpriteImageMode::Scale(SpriteScalingMode::FitEnd);

    // sprite.flip_x = true;
    sprite.flip_y = true;

    commands.spawn((sprite, Transform::from_translation(vec3(0.0, 0.0, 0.0))));

    let mut sprite = Sprite::from_image(texture_handle.clone());

    sprite.rect = Some(Rect {
        min: vec2(50.0, 55.0),
        max: vec2(150.0, 400.0),
    });
    sprite.custom_size = Some(vec2(300.0, 200.0));
    sprite.image_mode = SpriteImageMode::Scale(SpriteScalingMode::FitEnd);

    // sprite.flip_x = true;
    sprite.flip_y = true;

    commands.spawn((sprite, Transform::from_translation(vec3(0.0, 0.0, 0.0))));
}
