use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::{Query, Res};
use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectDeserialize, TypeUuid};
use bevy_render2::texture::Image;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, TypeUuid, Reflect)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
#[repr(C)]
pub struct Sprite {
    pub size: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
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
            flip_x: false,
            flip_y: false,
        }
    }
}

/// System that resizes sprites that have their resize mode set to automatic
pub fn sprite_auto_resize_system(
    textures: Res<Assets<Image>>,
    mut query: Query<(&mut Sprite, &Handle<Image>)>,
) {
    for (mut sprite, image_handle) in query.iter_mut() {
        match sprite.resize_mode {
            SpriteResizeMode::Manual => continue,
            SpriteResizeMode::Automatic => {
                if let Some(image) = textures.get(image_handle) {
                    let extent = image.texture_descriptor.size;
                    let texture_size = Vec2::new(extent.width as f32, extent.height as f32);
                    // only set sprite size if it has changed (this check prevents change
                    // detection from triggering)
                    if sprite.size != texture_size {
                        sprite.size = texture_size;
                    }
                }
            }
        }
    }
}
