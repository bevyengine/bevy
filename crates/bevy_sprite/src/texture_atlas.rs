use bevy_asset::{Asset, AssetId, Assets, Handle};
use bevy_ecs::component::Component;
use bevy_math::{URect, UVec2};
use bevy_reflect::Reflect;
use bevy_render::texture::Image;
use bevy_utils::HashMap;

/// Stores a map used to lookup the position of a texture in a [`TextureAtlas`].
/// This can be used to either use and look up a specific section of a texture, or animate frame-by-frame as a sprite sheet.
///
/// Optionally it can store a mapping from sub texture handles to the related area index (see
/// [`TextureAtlasBuilder`]).
///
/// [Example usage animating sprite.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_sheet.rs)
/// [Example usage animating sprite in response to an event.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_animation.rs)
/// [Example usage loading sprite sheet.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs)
///
/// [`TextureAtlasBuilder`]: crate::TextureAtlasBuilder
#[derive(Asset, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct TextureAtlasLayout {
    // TODO: add support to Uniforms derive to write dimensions and sprites to the same buffer
    pub size: UVec2,
    /// The specific areas of the atlas where each texture can be found
    pub textures: Vec<URect>,
    /// Maps from a specific image handle to the index in `textures` where they can be found.
    ///
    /// This field is set by [`TextureAtlasBuilder`].
    ///
    /// [`TextureAtlasBuilder`]: crate::TextureAtlasBuilder
    pub(crate) texture_handles: Option<HashMap<AssetId<Image>, usize>>,
}

/// Component used to draw a specific section of a texture.
///
/// It stores a handle to [`TextureAtlasLayout`] and the index of the current section of the atlas.
/// The texture atlas contains various *sections* of a given texture, allowing users to have a single
/// image file for either sprite animation or global mapping.
/// You can change the texture [`index`](Self::index) of the atlas to animate the sprite or display only a *section* of the texture
/// for efficient rendering of related game objects.
///
/// Check the following examples for usage:
/// - [`animated sprite sheet example`](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_sheet.rs)
/// - [`sprite animation event example`](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_animation.rs)
/// - [`texture atlas example`](https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs)
#[derive(Component, Default, Debug, Clone, Reflect)]
pub struct TextureAtlas {
    /// Texture atlas layout handle
    pub layout: Handle<TextureAtlasLayout>,
    /// Texture atlas section index
    pub index: usize,
}

impl TextureAtlasLayout {
    /// Create a new empty layout with custom `dimensions`
    pub fn new_empty(dimensions: UVec2) -> Self {
        Self {
            size: dimensions,
            texture_handles: None,
            textures: Vec::new(),
        }
    }

    /// Generate a [`TextureAtlasLayout`] as a grid where each
    /// `tile_size` by `tile_size` grid-cell is one of the *section* in the
    /// atlas. Grid cells are separated by some `padding`, and the grid starts
    /// at `offset` pixels from the top left corner. Resulting layout is
    /// indexed left to right, top to bottom.
    ///
    /// # Arguments
    ///
    /// * `tile_size` - Each layout grid cell size
    /// * `columns` - Grid column count
    /// * `rows` - Grid row count
    /// * `padding` - Optional padding between cells
    /// * `offset` - Optional global grid offset
    pub fn from_grid(
        tile_size: UVec2,
        columns: u32,
        rows: u32,
        padding: Option<UVec2>,
        offset: Option<UVec2>,
    ) -> Self {
        let padding = padding.unwrap_or_default();
        let offset = offset.unwrap_or_default();
        let mut sprites = Vec::new();
        let mut current_padding = UVec2::ZERO;

        for y in 0..rows {
            if y > 0 {
                current_padding.y = padding.y;
            }
            for x in 0..columns {
                if x > 0 {
                    current_padding.x = padding.x;
                }

                let cell = UVec2::new(x, y);
                let rect_min = (tile_size + current_padding) * cell + offset;

                sprites.push(URect {
                    min: rect_min,
                    max: rect_min + tile_size,
                });
            }
        }

        let grid_size = UVec2::new(columns, rows);

        Self {
            size: ((tile_size + current_padding) * grid_size) - current_padding,
            textures: sprites,
            texture_handles: None,
        }
    }

    /// Add a *section* to the list in the layout and returns its index
    /// which can be used with [`TextureAtlas`]
    ///
    /// # Arguments
    ///
    /// * `rect` - The section of the texture to be added
    ///
    /// [`TextureAtlas`]: crate::TextureAtlas
    pub fn add_texture(&mut self, rect: URect) -> usize {
        self.textures.push(rect);
        self.textures.len() - 1
    }

    /// The number of textures in the [`TextureAtlasLayout`]
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// Retrieves the texture *section* index of the given `texture` handle.
    ///
    /// This requires the layout to have been built using a [`TextureAtlasBuilder`]
    ///
    /// [`TextureAtlasBuilder`]: crate::TextureAtlasBuilder
    pub fn get_texture_index(&self, texture: impl Into<AssetId<Image>>) -> Option<usize> {
        let id = texture.into();
        self.texture_handles
            .as_ref()
            .and_then(|texture_handles| texture_handles.get(&id).cloned())
    }
}

impl TextureAtlas {
    /// Retrieves the current texture [`URect`] of the sprite sheet according to the section `index`
    pub fn texture_rect(&self, texture_atlases: &Assets<TextureAtlasLayout>) -> Option<URect> {
        let atlas = texture_atlases.get(&self.layout)?;
        atlas.textures.get(self.index).copied()
    }
}

impl From<Handle<TextureAtlasLayout>> for TextureAtlas {
    fn from(texture_atlas: Handle<TextureAtlasLayout>) -> Self {
        Self {
            layout: texture_atlas,
            index: 0,
        }
    }
}
