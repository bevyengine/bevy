use crate::Rect;
use bevy_asset::Handle;
use bevy_core::Bytes;
use bevy_ecs::component::Component;
use bevy_math::Vec2;
use bevy_reflect::TypeUuid;
use bevy_render::{
    color::Color,
    renderer::{RenderResource, RenderResourceType, RenderResources},
    texture::Texture,
};
use bevy_utils::HashMap;

/// An atlas containing multiple textures (like a spritesheet or a tilemap).
/// [Example usage animating sprite.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_sheet.rs)
/// [Example usage loading sprite sheet.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs)
#[derive(Debug, RenderResources, TypeUuid)]
#[uuid = "946dacc5-c2b2-4b30-b81d-af77d79d1db7"]
pub struct TextureAtlas {
    /// The handle to the texture in which the sprites are stored
    pub texture: Handle<Texture>,
    // TODO: add support to Uniforms derive to write dimensions and sprites to the same buffer
    pub size: Vec2,
    /// The specific areas of the atlas where each texture can be found
    #[render_resources(buffer)]
    pub textures: Vec<Rect>,
    #[render_resources(ignore)]
    pub texture_handles: Option<HashMap<Handle<Texture>, usize>>,
}

#[derive(Component, Debug, Clone, RenderResources)]
#[render_resources(from_self)]
#[repr(C)]
pub struct TextureAtlasSprite {
    pub color: Color,
    pub index: u32,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl RenderResource for TextureAtlasSprite {
    fn resource_type(&self) -> Option<RenderResourceType> {
        Some(RenderResourceType::Buffer)
    }

    fn buffer_byte_len(&self) -> Option<usize> {
        Some(24)
    }

    fn write_buffer_bytes(&self, buffer: &mut [u8]) {
        // Write the color buffer
        let (color_buf, rest) = buffer.split_at_mut(16);
        self.color.write_bytes(color_buf);

        // Write the index buffer
        let (index_buf, flip_buf) = rest.split_at_mut(4);
        self.index.write_bytes(index_buf);

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
    pub fn new(index: u32) -> TextureAtlasSprite {
        Self {
            index,
            ..Default::default()
        }
    }
}

impl TextureAtlas {
    /// Create a new `TextureAtlas` that has a texture, but does not have
    /// any individual sprites specified
    pub fn new_empty(texture: Handle<Texture>, dimensions: Vec2) -> Self {
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
        texture: Handle<Texture>,
        tile_size: Vec2,
        columns: usize,
        rows: usize,
    ) -> TextureAtlas {
        Self::from_grid_with_padding(texture, tile_size, columns, rows, Vec2::new(0f32, 0f32))
    }

    /// Generate a `TextureAtlas` by splitting a texture into a grid where each
    /// cell of the grid of `tile_size` is one of the textures in the atlas and is separated by
    /// some `padding` in the texture. The padding is assumed to be only between tiles
    /// and not at the borders of the texture.
    pub fn from_grid_with_padding(
        texture: Handle<Texture>,
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
    pub fn add_texture(&mut self, rect: Rect) -> u32 {
        self.textures.push(rect);
        (self.textures.len() - 1) as u32
    }

    /// How many textures are in the `TextureAtlas`
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    pub fn get_texture_index(&self, texture: &Handle<Texture>) -> Option<usize> {
        self.texture_handles
            .as_ref()
            .and_then(|texture_handles| texture_handles.get(texture).cloned())
    }
}
