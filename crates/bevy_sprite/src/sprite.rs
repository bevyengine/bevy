use crate::ColorMaterial;
use bevy_asset::{Assets, Handle};
use bevy_ecs::{Query, Res};
use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectDeserialize, TypeUuid};
use bevy_render::{renderer::RenderResources, texture::Texture};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, RenderResources, TypeUuid, Reflect)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
pub struct Sprite {
    pub size: Vec2,
    #[render_resources(ignore)]
    pub resize_mode: SpriteResizeMode,
}

/// Determines how `Sprite` resize should be handled
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
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
    for (mut sprite, handle) in query.iter_mut() {
        match sprite.resize_mode {
            SpriteResizeMode::Manual => continue,
            SpriteResizeMode::Automatic => {
                let material = materials.get(handle).unwrap();
                if let Some(ref texture_handle) = material.texture {
                    if let Some(texture) = textures.get(texture_handle) {
                        let texture_size = texture.size.as_vec3().truncate();
                        // only set sprite size if it has changed (this check prevents change detection from triggering)
                        if sprite.size != texture_size {
                            sprite.size = texture_size;
                        }
                    }
                }
            }
        }
    }
}
