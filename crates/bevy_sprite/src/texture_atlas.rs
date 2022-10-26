use crate::Anchor;
use bevy_asset::Handle;
use bevy_ecs::component::Component;
use bevy_math::{Rect, Vec2};
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

#[derive(Component, Debug, Clone, Reflect)]
pub struct TextureAtlasSprite {
    pub color: Color,
    pub index: usize,
    pub flip_x: bool,
    pub flip_y: bool,
    /// An optional custom size for the sprite that will be used when rendering, instead of the size
    /// of the sprite's image in the atlas
    pub custom_size: Option<Vec2>,
    pub anchor: Anchor,
}

impl Default for TextureAtlasSprite {
    fn default() -> Self {
        Self {
            index: 0,
            color: Color::WHITE,
            flip_x: false,
            flip_y: false,
            custom_size: None,
            anchor: Anchor::default(),
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
    /// `tile_size` by `tile_size` grid-cell is one of the textures in the
    /// atlas. Grid cells are separated by some `padding`, and the grid starts
    /// at `offset` pixels from the top left corner. Resulting `TextureAtlas` is
    /// indexed left to right, top to bottom.
    pub fn from_grid(
        texture: Handle<Image>,
        tile_size: Vec2,
        columns: usize,
        rows: usize,
        padding: Option<Vec2>,
        offset: Option<Vec2>,
    ) -> TextureAtlas {
        let padding = padding.unwrap_or_default();
        let offset = offset.unwrap_or_default();
        let mut sprites = Vec::new();
        let mut current_padding = Vec2::ZERO;

        for y in 0..rows {
            if y > 0 {
                current_padding.y = padding.y;
            }
            for x in 0..columns {
                if x > 0 {
                    current_padding.x = padding.x;
                }

                let cell = Vec2::new(x as f32, y as f32);

                let rect_min = (tile_size + current_padding) * cell + offset;

                sprites.push(Rect {
                    min: rect_min,
                    max: rect_min + tile_size,
                });
            }
        }

        let grid_size = Vec2::new(columns as f32, rows as f32);

        TextureAtlas {
            size: ((tile_size + current_padding) * grid_size) - current_padding,
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
