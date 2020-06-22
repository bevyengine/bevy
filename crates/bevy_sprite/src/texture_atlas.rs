use crate::Rect;
use bevy_asset::Handle;
use bevy_core::bytes::Bytes;
use bevy_render::{
    render_resource::{RenderResource, RenderResources},
    texture::Texture,
    Color,
};
use glam::{Vec2, Vec3};
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
    pub position: Vec3,
    pub color: Color,
    pub scale: f32,
    pub index: u32,
}

impl Default for TextureAtlasSprite {
    fn default() -> Self {
        Self {
            index: 0,
            color: Color::WHITE,
            scale: 1.0,
            position: Default::default(),
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

    pub fn get_texture_index(&self, texture: Handle<Texture>) -> Option<usize> {
        self.texture_handles
            .as_ref()
            .and_then(|texture_handles| texture_handles.get(&texture).cloned())
    }
}

#[cfg(test)]
mod tests {
    use crate::TextureAtlasSprite;
    use bevy_core::bytes::{Bytes, FromBytes};
    use bevy_render::Color;
    use glam::Vec3;

    #[test]
    fn test_atlas_byte_conversion() {
        let x = TextureAtlasSprite {
            color: Color::RED,
            index: 2,
            position: Vec3::new(1., 2., 3.),
            scale: 4.0,
        };

        assert_eq!(x.byte_len(), 36);
        let mut bytes = vec![0; x.byte_len()];

        x.write_bytes(&mut bytes);

        let position = Vec3::from_bytes(&bytes[0..12]);
        let color = Color::from_bytes(&bytes[12..28]);
        let scale = f32::from_bytes(&bytes[28..32]);
        let index = u32::from_bytes(&bytes[32..36]);

        assert_eq!(position, x.position);
        assert_eq!(color, x.color);
        assert_eq!(scale, x.scale);
        assert_eq!(index, x.index);
    }
}
