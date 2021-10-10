use crate::ColorMaterial;
use bevy_asset::{Assets, Handle};
use bevy_core::Bytes;
use bevy_ecs::{
    component::Component,
    query::Without,
    system::{Query, Res},
};
use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectDeserialize, TypeUuid};
use bevy_render::{
    draw::OutsideFrustum,
    renderer::{RenderResource, RenderResourceType, RenderResources},
    texture::Texture,
};
use serde::{Deserialize, Serialize};

/// General Sprite Examples: [Link](https://github.com/bevyengine/bevy/tree/latest/examples/2d)
#[derive(Component, Debug, Default, Clone, TypeUuid, Reflect, RenderResources)]
#[render_resources(from_self)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
#[repr(C)]
pub struct Sprite {
    pub size: Vec2,
    /// When true flips sprite to left. [Example](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_flipping.rs)
    pub flip_x: bool,
    /// When true flips sprite upside down. [Example](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_flipping.rs)
    pub flip_y: bool,
    pub resize_mode: SpriteResizeMode,
}

impl RenderResource for Sprite {
    fn resource_type(&self) -> Option<RenderResourceType> {
        Some(RenderResourceType::Buffer)
    }

    fn buffer_byte_len(&self) -> Option<usize> {
        Some(12)
    }

    fn write_buffer_bytes(&self, buffer: &mut [u8]) {
        // Write the size buffer
        let (size_buf, flip_buf) = buffer.split_at_mut(8);
        self.size.write_bytes(size_buf);

        // First bit means flip x, second bit means flip y
        flip_buf[0] = if self.flip_x { 0b01 } else { 0 } | if self.flip_y { 0b10 } else { 0 };
        flip_buf[1] = 0;
        flip_buf[2] = 0;
        flip_buf[3] = 0;
    }

    fn texture(&self) -> Option<&Handle<Texture>> {
        None
    }
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

pub fn sprite_system(
    materials: Res<Assets<ColorMaterial>>,
    textures: Res<Assets<Texture>>,
    mut query: Query<(&mut Sprite, &Handle<ColorMaterial>), Without<OutsideFrustum>>,
) {
    for (mut sprite, handle) in query.iter_mut() {
        match sprite.resize_mode {
            SpriteResizeMode::Manual => continue,
            SpriteResizeMode::Automatic => {
                let material = materials.get(handle).unwrap();
                if let Some(ref texture_handle) = material.texture {
                    if let Some(texture) = textures.get(texture_handle) {
                        let texture_size = texture.size.as_vec3().truncate();
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
}
