//! Displays a single [`Sprite`], created from an image.

use bevy::prelude::*;
use bevy_internal::{
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{SpriteMaterial, SpriteMaterialPlugin, SpriteWithMaterial, SpriteWithMaterialBundle},
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
        sprite: SpriteWithMaterial {
            material: sprite_materials.add(GrayScale {}),
            ..default()
        },
        texture: asset_server.load("textures/rpg/chars/sensei/sensei.png"),
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
