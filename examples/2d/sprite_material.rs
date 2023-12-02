//! Displays a single [`Sprite`], created from an image.

use bevy::prelude::*;
use bevy_internal::{
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{SpriteMaterial, SpriteMaterialPlugin, SpriteWithMaterialBundle},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
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
    commands.spawn(SpriteWithMaterialBundle {
        sprite: Sprite {
            color: Color::rgb(0.21, 0.72, 0.07),
            ..default()
        },
        texture: asset_server.load("textures/rpg/chars/sensei/sensei.png"),
        material: sprite_materials.add(GrayScale {}),
        ..default()
    });
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct GrayScale {}

impl SpriteMaterial for GrayScale {
    fn fragment_shader() -> ShaderRef {
        "shaders/grayscale.wgsl".into()
    }
}
