use crate::Rect;
use bevy_asset::Handle;
use bevy_ecs::component::Component;
use bevy_math::Vec2;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::texture::Image;
use bevy_utils::HashMap;

/// An atlas containing multiple textures (like a spritesheet or a tilemap).
/// [Example usage animating sprite.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_sheet.rs)
/// [Example usage loading sprite sheet.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs)
#[derive(Reflect, FromReflect, Debug, Clone, TypeUuid)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
#[reflect(Debug)]
pub struct TextureAtlas {
    // TODO: add support to Uniforms derive to write dimensions and sprites to the same buffer
    pub size: Vec2,
    /// The specific areas of the atlas where each texture can be found
    pub textures: Vec<Rect>,
    /// Mapping from texture handle to index
    pub(crate) texture_handles: Option<HashMap<Handle<Image>, usize>>,
}

#[derive(Component, Default, Debug, Clone, Reflect)]
pub struct TextureSheetIndex(pub usize);

impl TextureSheetIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }
}

impl From<usize> for TextureSheetIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

impl TextureAtlas {
    /// Create a new [`TextureAtlas`] that has a texture, but does not have
    /// any individual sprites specified
    pub fn new_empty(dimensions: Vec2) -> Self {
        Self {
            size: dimensions,
            texture_handles: None,
            textures: Vec::new(),
        }
    }

    /// Generate a `TextureAtlas` by splitting a texture into a grid where each
    /// `tile_size` by `tile_size` grid-cell is one of the textures in the atlas
    pub fn from_grid(tile_size: Vec2, columns: usize, rows: usize) -> TextureAtlas {
        Self::from_grid_with_padding(tile_size, columns, rows, Vec2::ZERO, Vec2::ZERO)
    }

    /// Generate a `TextureAtlas` by splitting a texture into a grid where each
    /// `tile_size` by `tile_size` grid-cell is one of the textures in the
    /// atlas. Grid cells are separated by some `padding`, and the grid starts
    /// at `offset` pixels from the top left corner. The resulting [`TextureAtlas`] is
    /// indexed left to right, top to bottom.
    pub fn from_grid_with_padding(
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
            texture_handles: None,
        }
    }

    /// Add a sprite to the list of textures in the [`TextureAtlas`]
    /// returns an index to the texture which can be used with [`TextureAtlasSprite`]
    ///
    /// # Arguments
    ///
    /// * `rect` - The section of the atlas that contains the texture to be added,
    /// from the top-left corner of the texture to the bottom-right corner
    pub fn add_texture(&mut self, rect: Rect) -> usize {
        self.textures.push(rect);
        self.textures.len() - 1
    }

    /// The number of textures in the [`TextureAtlas`]
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// Returns `true` if there are no textures in the [`TextureAtlas`]
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// Returns the index of the texture corresponding to the given image handle in the [`TextureAtlas`]
    pub fn get_texture_index(&self, texture: &Handle<Image>) -> Option<usize> {
        self.texture_handles
            .as_ref()
            .and_then(|texture_handles| texture_handles.get(texture).cloned())
    }
}
