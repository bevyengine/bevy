use crate::Rect;
use bevy_asset::Handle;
use bevy_ecs::component::Component;
use bevy_math::Vec2;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{color::Color, texture::Image};
use bevy_utils::HashMap;

/// An atlas containing multiple textures (like a spritesheet or a tilemap).
/// [Example usage animating sprite.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_sheet.rs)
/// [Example usage loading sprite sheet.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs)
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
pub struct TextureAtlas {
    /// The handle to the texture in which the sprites are stored
    pub texture: Handle<Image>,
    // TODO: add support to Uniforms derive to write dimensions and sprites to the same buffer
    pub size: Vec2,
    /// The specific areas of the atlas where each texture can be found
    pub textures: Vec<Rect>,
    pub texture_handles: Option<HashMap<Handle<Image>, usize>>,
}

#[derive(Component, Debug, Clone, TypeUuid, Reflect)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
pub struct TextureAtlasSprite {
    pub color: Color,
    pub index: usize,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl Default for TextureAtlasSprite {
    fn default() -> Self {
        Self {
            index: 0,
            color: Color::WHITE,
            flip_x: false,
            flip_y: false,
        }
    }
}

impl TextureAtlasSprite {
    pub fn new(index: usize) -> TextureAtlasSprite {
        Self {
            index,
            ..Default::default()
        }
    }
}

impl TextureAtlas {
    /// Create a new `TextureAtlas` that has a texture, but does not have
    /// any individual sprites specified
    pub fn new_empty(texture: Handle<Image>, dimensions: Vec2) -> Self {
        Self {
            texture,
            size: dimensions,
            texture_handles: None,
            textures: Vec::new(),
        }
    }

    /// Generate a `TextureAtlas` by splitting a texture into a grid where each
    /// cell of the grid  of `tile_size` is one of the textures in the atlas
    pub fn from_grid(
        texture: Handle<Image>,
        tile_size: Vec2,
        columns: usize,
        rows: usize,
    ) -> TextureAtlas {
        Self::from_grid_with_padding(texture, tile_size, columns, rows, Vec2::new(0f32, 0f32))
    }

    /// Generate a `TextureAtlas` by splitting a texture into a grid where each
    /// cell of the grid of `tile_size` is one of the textures in the atlas and is separated by
    /// some `padding` in the texture
    pub fn from_grid_with_padding(
        texture: Handle<Image>,
        tile_size: Vec2,
        columns: usize,
        rows: usize,
        padding: Vec2,
    ) -> TextureAtlas {
        let mut sprites = Vec::new();
        let mut x_padding = 0.0;
        let mut y_padding = 0.0;

        for y in 0..rows {
            if y > 0 {
                y_padding = padding.y;
            }
            for x in 0..columns {
                if x > 0 {
                    x_padding = padding.x;
                }

                let rect_min = Vec2::new(
                    (tile_size.x + x_padding) * x as f32,
                    (tile_size.y + y_padding) * y as f32,
                );

                sprites.push(Rect {
                    min: rect_min,
                    max: Vec2::new(rect_min.x + tile_size.x, rect_min.y + tile_size.y),
                })
            }
        }

        TextureAtlas {
            size: Vec2::new(
                ((tile_size.x + x_padding) * columns as f32) - x_padding,
                ((tile_size.y + y_padding) * rows as f32) - y_padding,
            ),
            textures: sprites,
            texture,
            texture_handles: None,
        }
    }

    /// Add a sprite to the list of textures in the `TextureAtlas`
    /// returns an index to the texture which can be used with `TextureAtlasSprite`
    ///
    /// # Arguments
    ///
    /// * `rect` - The section of the atlas that contains the texture to be added,
    /// from the top-left corner of the texture to the bottom-right corner
    pub fn add_texture(&mut self, rect: Rect) -> usize {
        self.textures.push(rect);
        self.textures.len() - 1
    }

    /// How many textures are in the `TextureAtlas`
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    pub fn get_texture_index(&self, texture: &Handle<Image>) -> Option<usize> {
        self.texture_handles
            .as_ref()
            .and_then(|texture_handles| texture_handles.get(texture).cloned())
    }
}
