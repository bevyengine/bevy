use crate::Rect;
use bevy_asset::Handle;
use bevy_core::Bytes;
use bevy_math::Vec2;
use bevy_render::{
    color::Color,
    renderer::{RenderResource, RenderResources},
    texture::Texture,
};
use std::collections::HashMap;

#[derive(RenderResources)]
pub struct TextureAtlas {
    pub texture: Handle<Texture>,
    // TODO: add support to Uniforms derive to write dimensions and sprites to the same buffer
    pub size: Vec2,
    #[render_resources(buffer)]
    pub textures: Vec<Rect>,
    #[render_resources(ignore)]
    pub texture_handles: Option<HashMap<Handle<Texture>, usize>>,
}

// NOTE: cannot do `unsafe impl Byteable` here because Vec3 takes up the space of a Vec4. If/when glam changes this we can swap out
// Bytes for Byteable as a micro-optimization. https://github.com/bitshifter/glam-rs/issues/36
#[derive(Bytes, RenderResources, RenderResource)]
#[render_resources(from_self)]
pub struct TextureAtlasSprite {
    pub color: Color,
    pub index: u32,
}

impl Default for TextureAtlasSprite {
    fn default() -> Self {
        Self {
            index: 0,
            color: Color::WHITE,
        }
    }
}

impl TextureAtlasSprite {
    pub fn new(index: u32) -> TextureAtlasSprite {
        Self {
            index,
            ..Default::default()
        }
    }
}

impl TextureAtlas {
    pub fn new_empty(texture: Handle<Texture>, dimensions: Vec2) -> Self {
        Self {
            texture,
            size: dimensions,
            texture_handles: None,
            textures: Vec::new(),
        }
    }

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
            size,
            textures: sprites,
            texture,
            texture_handles: None,
        }
    }

    pub fn add_texture(&mut self, rect: Rect) {
        self.textures.push(rect);
    }

    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    pub fn get_texture_index(&self, texture: Handle<Texture>) -> Option<usize> {
        self.texture_handles
            .as_ref()
            .and_then(|texture_handles| texture_handles.get(&texture).cloned())
    }
}
