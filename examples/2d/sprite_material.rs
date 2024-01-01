//! Displays a single [`Sprite`], created from an image, and applies a grayscale effect to it.

use bevy::prelude::*;
use bevy_internal::{
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{SpriteMaterial, SpriteMaterialPlugin, SpriteWithMaterialBundle},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the grayscale material plugin to the app
        .add_plugins(SpriteMaterialPlugin::<GrayScale>::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut sprite_materials: ResMut<Assets<GrayScale>>,
) {
    commands.spawn(Camera2dBundle::default());

    // Create a sprite with a grayscale material
    commands.spawn(SpriteWithMaterialBundle {
        texture: asset_server.load("textures/rpg/chars/sensei/sensei.png"),
        material: sprite_materials.add(GrayScale {}),
        ..default()
    });
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct GrayScale {}

impl SpriteMaterial for GrayScale {
    fn fragment_shader() -> ShaderRef {
        // Return the shader reference for the grayscale fragment shader
        "shaders/grayscale.wgsl".into()
    }
}
