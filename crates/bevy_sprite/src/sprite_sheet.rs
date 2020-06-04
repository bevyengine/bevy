use crate::Rect;
use bevy_asset::Handle;
use bevy_derive::{Bytes, Uniform, Uniforms};
use bevy_render::texture::Texture;
use glam::{Vec2, Vec3};

#[derive(Uniforms)]
pub struct SpriteSheet {
    pub texture: Handle<Texture>,
    // TODO: add support to Uniforms derive to write dimensions and sprites to the same buffer
    pub dimensions: Vec2,
    #[uniform(buffer)]
    pub sprites: Vec<Rect>,
}

// NOTE: cannot do `unsafe impl Byteable` here because Vec3 takes up the space of a Vec4. If/when glam changes this we can swap out
// Bytes for Byteable as a micro-optimization. https://github.com/bitshifter/glam-rs/issues/36
#[derive(Uniform, Bytes, Default)]
pub struct SpriteSheetSprite {
    pub position: Vec3,
    pub scale: f32,
    pub index: u32,
}

impl SpriteSheet {
    pub fn from_grid(
        texture: Handle<Texture>,
        size: Vec2,
        columns: usize,
        rows: usize,
    ) -> SpriteSheet {
        let sprite_width = size.x() / columns as f32;
        let sprite_height = size.y() / rows as f32;
        let mut sprites = Vec::new();
        for y in 0..rows {
            for x in 0..columns {
                sprites.push(Rect {
                    min: Vec2::new(x as f32 * sprite_width, y as f32 * sprite_height),
                    max: Vec2::new(
                        (x + 1) as f32 * sprite_width,
                        (y + 1) as f32 * sprite_height,
                    ),
                })
            }
        }
        SpriteSheet {
            dimensions: size,
            sprites,
            texture,
        }
    }
}
