use crate::TextureAtlas;
use bevy_asset::Assets;
use bevy_math::{IVec2, Rect, Vec2};
use bevy_render::texture::{Image, TextureFormatPixelInfo};
use guillotiere::{size2, Allocation, AtlasAllocator};

/// Helper utility to update [`TextureAtlas`] on the fly.
///
/// Helpful in cases when texture is created procedurally,
/// e.g: in a font glyph [`TextureAtlas`], only add the [`Image`] texture for letters to be rendered.
pub struct DynamicTextureAtlasBuilder {
    atlas_allocator: AtlasAllocator,
    padding: i32,
}

impl DynamicTextureAtlasBuilder {
    /// Create a new [`DynamicTextureAtlasBuilder`]
    ///
    /// # Arguments
    ///
    /// * `size` - total size for the atlas
    /// * `padding` - gap added between textures in the atlas, both in x axis and y axis
    pub fn new(size: Vec2, padding: i32) -> Self {
        Self {
            atlas_allocator: AtlasAllocator::new(to_size2(size)),
            padding,
        }
    }

    /// Add a new texture to [`TextureAtlas`].
    /// It is user's responsibility to pass in the correct [`TextureAtlas`]
    pub fn add_texture(
        &mut self,
        texture_atlas: &mut TextureAtlas,
        textures: &mut Assets<Image>,
        texture: &Image,
    ) -> Option<usize> {
        let allocation = self.atlas_allocator.allocate(size2(
            texture.texture_descriptor.size.width as i32 + self.padding,
            texture.texture_descriptor.size.height as i32 + self.padding,
        ));
        if let Some(allocation) = allocation {
            let atlas_texture = textures.get_mut(&texture_atlas.texture).unwrap();
            self.place_texture(atlas_texture, allocation, texture);
            let mut rect: Rect = to_rect(allocation.rectangle);
            rect.max -= self.padding as f32;
            Some(texture_atlas.add_texture(rect))
        } else {
            None
        }
    }

    fn place_texture(
        &mut self,
        atlas_texture: &mut Image,
        allocation: Allocation,
        texture: &Image,
    ) {
        let mut rect = allocation.rectangle;
        rect.max.x -= self.padding;
        rect.max.y -= self.padding;
        let atlas_width = atlas_texture.texture_descriptor.size.width as usize;
        let rect_width = rect.width() as usize;
        let format_size = atlas_texture.texture_descriptor.format.pixel_size();

        for (texture_y, bound_y) in (rect.min.y..rect.max.y).map(|i| i as usize).enumerate() {
            let begin = (bound_y * atlas_width + rect.min.x as usize) * format_size;
            let end = begin + rect_width * format_size;
            let texture_begin = texture_y * rect_width * format_size;
            let texture_end = texture_begin + rect_width * format_size;
            atlas_texture.data[begin..end]
                .copy_from_slice(&texture.data[texture_begin..texture_end]);
        }
    }
}

fn to_rect(rectangle: guillotiere::Rectangle) -> Rect {
    Rect {
        min: IVec2::new(rectangle.min.x, rectangle.min.y).as_vec2(),
        max: IVec2::new(rectangle.max.x, rectangle.max.y).as_vec2(),
    }
}

fn to_size2(vec2: Vec2) -> guillotiere::Size {
    guillotiere::Size::new(vec2.x as i32, vec2.y as i32)
}
