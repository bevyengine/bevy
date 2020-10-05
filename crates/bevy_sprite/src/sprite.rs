use crate::ColorMaterial;
use bevy_asset::{Assets, Handle};
use bevy_ecs::{Query, Res};
use bevy_math::Vec2;
use bevy_render::{renderer::RenderResources, texture::Texture};

#[derive(Debug, Default, RenderResources)]
pub struct Sprite {
    pub size: Vec2,
    #[render_resources(ignore)]
    pub resize_mode: SpriteResizeMode,
}

/// Determines how `Sprite` resize should be handled
#[derive(Debug)]
pub enum SpriteResizeMode {
    Manual,
    Automatic,
}

impl Default for SpriteResizeMode {
    fn default() -> Self {
        SpriteResizeMode::Automatic
    }
}

impl Sprite {
    /// Creates new `Sprite` with `SpriteResizeMode::Manual` value for `resize_mode`
    pub fn new(size: Vec2) -> Self {
        Self {
            size,
            resize_mode: SpriteResizeMode::Manual,
        }
    }
}

pub fn sprite_system(
    materials: Res<Assets<ColorMaterial>>,
    textures: Res<Assets<Texture>>,
    mut query: Query<(&mut Sprite, &Handle<ColorMaterial>)>,
) {
    for (mut sprite, handle) in &mut query.iter() {
        match sprite.resize_mode {
            SpriteResizeMode::Manual => continue,
            SpriteResizeMode::Automatic => {
                let material = materials.get(&handle).unwrap();
                if let Some(texture_handle) = material.texture {
                    if let Some(texture) = textures.get(&texture_handle) {
                        sprite.size = texture.size;
                    }
                }
            }
        }
    }
}
