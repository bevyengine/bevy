use crate::Rect;
use bevy_asset::Handle;
use bevy_derive::{Bytes, Uniform, Uniforms};
use bevy_render::texture::Texture;
use glam::{Vec2, Vec3};
use std::collections::HashMap;

#[derive(Uniforms)]
pub struct TextureAtlas {
    pub texture: Handle<Texture>,
    // TODO: add support to Uniforms derive to write dimensions and sprites to the same buffer
    pub dimensions: Vec2,
    #[uniform(buffer)]
    pub textures: Vec<Rect>,
    #[uniform(ignore)]
    pub texture_handles: Option<HashMap<Handle<Texture>, usize>>,
}

// NOTE: cannot do `unsafe impl Byteable` here because Vec3 takes up the space of a Vec4. If/when glam changes this we can swap out
// Bytes for Byteable as a micro-optimization. https://github.com/bitshifter/glam-rs/issues/36
#[derive(Uniform, Bytes, Default)]
pub struct TextureAtlasSprite {
    pub position: Vec3,
    pub scale: f32,
    pub index: u32,
}

impl TextureAtlas {
    pub fn from_grid(
        texture: Handle<Texture>,
        size: Vec2,
        columns: usize,
        rows: usize,
    ) -> TextureAtlas {
        let texture_width = size.x() / columns as f32;
        let texture_height = size.y() / rows as f32;
        let mut sprites = Vec::new();
        for y in 0..rows {
            for x in 0..columns {
                sprites.push(Rect {
                    min: Vec2::new(x as f32 * texture_width, y as f32 * texture_height),
                    max: Vec2::new(
                        (x + 1) as f32 * texture_width,
                        (y + 1) as f32 * texture_height,
                    ),
                })
            }
        }
        TextureAtlas {
            dimensions: size,
            textures: sprites,
            texture,
            texture_handles: None,
        }
    }

    pub fn get_texture_index(&self, texture: Handle<Texture>) -> Option<usize> {
        self.texture_handles
            .as_ref()
            .and_then(|texture_handles| texture_handles.get(&texture).cloned())
    }
}
