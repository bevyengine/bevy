use crate::{Image, TextureAccessError, TextureAtlasLayout, TextureFormatPixelInfo as _};
use bevy_asset::RenderAssetUsages;
use bevy_math::{URect, UVec2};
use guillotiere::{size2, Allocation, AtlasAllocator};
use thiserror::Error;

/// An error produced by [`DynamicTextureAtlasBuilder`] when trying to add a new
/// texture to a [`TextureAtlasLayout`].
#[derive(Debug, Error)]
pub enum DynamicTextureAtlasBuilderError {
    /// Unable to allocate space within the atlas for the new texture
    #[error("Couldn't allocate space to add the image requested")]
    FailedToAllocateSpace,
    /// Attempted to add a texture to an uninitialized atlas
    #[error("cannot add texture to uninitialized atlas texture")]
    UninitializedAtlas,
    /// Attempted to add an uninitialized texture to an atlas
    #[error("cannot add uninitialized texture to atlas")]
    UninitializedSourceTexture,
    /// A texture access error occurred
    #[error("texture access error: {0}")]
    TextureAccess(#[from] TextureAccessError),
}

/// Helper utility to update [`TextureAtlasLayout`] on the fly.
///
/// Helpful in cases when texture is created procedurally,
/// e.g: in a font glyph [`TextureAtlasLayout`], only add the [`Image`] texture for letters to be rendered.
pub struct DynamicTextureAtlasBuilder {
    atlas_allocator: AtlasAllocator,
    padding: u32,
}

impl DynamicTextureAtlasBuilder {
    /// Create a new [`DynamicTextureAtlasBuilder`]
    ///
    /// # Arguments
    ///
    /// * `size` - total size for the atlas
    /// * `padding` - gap added between textures in the atlas (and the atlas edge), both in x axis
    ///   and y axis
    pub fn new(size: UVec2, padding: u32) -> Self {
        // This doesn't need to be >= since `AtlasAllocator` requires non-zero size.
        debug_assert!(size.x > padding && size.y > padding);
        Self {
            // Leave out padding at the right and bottom, so we don't put textures on the edge of
            // atlas.
            atlas_allocator: AtlasAllocator::new(to_size2(size - padding)),
            padding,
        }
    }

    /// Add a new texture to `atlas_layout`.
    ///
    /// It is the user's responsibility to pass in the correct [`TextureAtlasLayout`].
    /// Also, the asset that `atlas_texture_handle` points to must have a usage matching
    /// [`RenderAssetUsages::MAIN_WORLD`].
    ///
    /// # Arguments
    ///
    /// * `atlas_layout` - The atlas layout to add the texture to.
    /// * `texture` - The source texture to add to the atlas.
    /// * `atlas_texture` - The destination atlas texture to copy the source texture to.
    pub fn add_texture(
        &mut self,
        atlas_layout: &mut TextureAtlasLayout,
        texture: &Image,
        atlas_texture: &mut Image,
    ) -> Result<usize, DynamicTextureAtlasBuilderError> {
        // Allocate enough space for the texture and the padding to the top and left (bottom and
        // right padding are taken care off since the allocator size omits it on creation).
        let allocation = self.atlas_allocator.allocate(size2(
            (texture.width() + self.padding).try_into().unwrap(),
            (texture.height() + self.padding).try_into().unwrap(),
        ));
        if let Some(mut allocation) = allocation {
            assert!(
                atlas_texture.asset_usage.contains(RenderAssetUsages::MAIN_WORLD),
                "The atlas_texture image must have the RenderAssetUsages::MAIN_WORLD usage flag set"
            );
            let rect = &mut allocation.rectangle;
            // Remove the padding from the top and left (bottom and right padding is taken care of
            // by the "next" allocation and the border restriction).
            rect.min.x += self.padding as i32;
            rect.min.y += self.padding as i32;

            self.place_texture(atlas_texture, allocation, texture)?;
            Ok(atlas_layout.add_texture(to_rect(allocation.rectangle)))
        } else {
            Err(DynamicTextureAtlasBuilderError::FailedToAllocateSpace)
        }
    }

    fn place_texture(
        &mut self,
        atlas_texture: &mut Image,
        allocation: Allocation,
        texture: &Image,
    ) -> Result<(), DynamicTextureAtlasBuilderError> {
        let rect = &allocation.rectangle;
        let atlas_width = atlas_texture.width() as usize;
        let rect_width = rect.width() as usize;
        let format_size = atlas_texture.texture_descriptor.format.pixel_size()?;

        let Some(ref mut atlas_data) = atlas_texture.data else {
            return Err(DynamicTextureAtlasBuilderError::UninitializedAtlas);
        };
        let Some(ref data) = texture.data else {
            return Err(DynamicTextureAtlasBuilderError::UninitializedSourceTexture);
        };
        for (texture_y, bound_y) in (rect.min.y..rect.max.y).map(|i| i as usize).enumerate() {
            let begin = (bound_y * atlas_width + rect.min.x as usize) * format_size;
            let end = begin + rect_width * format_size;
            let texture_begin = texture_y * rect_width * format_size;
            let texture_end = texture_begin + rect_width * format_size;
            atlas_data[begin..end].copy_from_slice(&data[texture_begin..texture_end]);
        }
        Ok(())
    }
}

fn to_rect(rectangle: guillotiere::Rectangle) -> URect {
    URect {
        min: UVec2::new(
            rectangle.min.x.try_into().unwrap(),
            rectangle.min.y.try_into().unwrap(),
        ),
        max: UVec2::new(
            rectangle.max.x.try_into().unwrap(),
            rectangle.max.y.try_into().unwrap(),
        ),
    }
}

fn to_size2(vec2: UVec2) -> guillotiere::Size {
    guillotiere::Size::new(vec2.x as i32, vec2.y as i32)
}

#[cfg(test)]
mod tests {
    use bevy_asset::RenderAssetUsages;
    use bevy_math::{URect, UVec2};

    use crate::{DynamicTextureAtlasBuilder, Image, TextureAtlasLayout};

    fn make_filled_image(size: UVec2, pixel_rgba_bytes: [u8; 4]) -> Image {
        Image::new_fill(
            wgpu_types::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            wgpu_types::TextureDimension::D2,
            &pixel_rgba_bytes,
            wgpu_types::TextureFormat::Rgba8Unorm,
            RenderAssetUsages::all(),
        )
    }

    fn rect_contains_value(image: &Image, rect: URect, pixel_rgba_bytes: [u8; 4]) -> bool {
        let image_data = image.data.as_ref().unwrap();
        for y in rect.min.y..rect.max.y {
            for x in rect.min.x..rect.max.x {
                let byte_start = ((x + y * image.width()) * 4) as usize;
                if image_data[byte_start..(byte_start + 4)] != pixel_rgba_bytes {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn allocate_textures() {
        let size = UVec2::new(30, 30);

        let mut atlas_texture = make_filled_image(size, [0, 0, 0, 0]);
        let mut layout = TextureAtlasLayout::new_empty(size);
        let mut builder = DynamicTextureAtlasBuilder::new(size, 0);

        let square = UVec2::new(10, 10);
        let colors = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 0, 255, 255],
            [0, 255, 255, 255],
            [0, 255, 255, 255],
        ];
        let texture_0 = builder
            .add_texture(
                &mut layout,
                &make_filled_image(square, colors[0]),
                &mut atlas_texture,
            )
            .unwrap();
        let texture_1 = builder
            .add_texture(
                &mut layout,
                &make_filled_image(square, colors[1]),
                &mut atlas_texture,
            )
            .unwrap();
        let texture_2 = builder
            .add_texture(
                &mut layout,
                &make_filled_image(square, colors[2]),
                &mut atlas_texture,
            )
            .unwrap();
        let texture_3 = builder
            .add_texture(
                &mut layout,
                &make_filled_image(square, colors[3]),
                &mut atlas_texture,
            )
            .unwrap();
        let texture_4 = builder
            .add_texture(
                &mut layout,
                &make_filled_image(square, colors[4]),
                &mut atlas_texture,
            )
            .unwrap();
        let texture_5 = builder
            .add_texture(
                &mut layout,
                &make_filled_image(square, colors[5]),
                &mut atlas_texture,
            )
            .unwrap();

        let expected_rects = [
            URect::from_corners(UVec2::new(0, 0), UVec2::new(10, 10)),
            URect::from_corners(UVec2::new(10, 0), UVec2::new(20, 10)),
            URect::from_corners(UVec2::new(20, 0), UVec2::new(30, 10)),
            URect::from_corners(UVec2::new(0, 10), UVec2::new(10, 20)),
            URect::from_corners(UVec2::new(0, 20), UVec2::new(10, 30)),
            URect::from_corners(UVec2::new(10, 10), UVec2::new(20, 20)),
        ];
        assert_eq!(layout.textures[texture_0], expected_rects[0]);
        assert_eq!(layout.textures[texture_1], expected_rects[1]);
        assert_eq!(layout.textures[texture_2], expected_rects[2]);
        assert_eq!(layout.textures[texture_3], expected_rects[3]);
        assert_eq!(layout.textures[texture_4], expected_rects[4]);
        assert_eq!(layout.textures[texture_5], expected_rects[5]);

        assert!(rect_contains_value(
            &atlas_texture,
            expected_rects[0],
            colors[0]
        ));
        assert!(rect_contains_value(
            &atlas_texture,
            expected_rects[1],
            colors[1]
        ));
        assert!(rect_contains_value(
            &atlas_texture,
            expected_rects[2],
            colors[2]
        ));
        assert!(rect_contains_value(
            &atlas_texture,
            expected_rects[3],
            colors[3]
        ));
        assert!(rect_contains_value(
            &atlas_texture,
            expected_rects[4],
            colors[4]
        ));
        assert!(rect_contains_value(
            &atlas_texture,
            expected_rects[5],
            colors[5]
        ));
    }

    #[test]
    fn allocate_textures_with_padding() {
        let size = UVec2::new(12, 12);

        let mut atlas_texture = make_filled_image(size, [0, 0, 0, 0]);
        let mut layout = TextureAtlasLayout::new_empty(size);
        let mut builder = DynamicTextureAtlasBuilder::new(size, 1);

        let square = UVec2::new(3, 3);
        let colors = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]];
        let texture_0 = builder
            .add_texture(
                &mut layout,
                &make_filled_image(square, colors[0]),
                &mut atlas_texture,
            )
            .unwrap();
        let texture_1 = builder
            .add_texture(
                &mut layout,
                &make_filled_image(square, colors[1]),
                &mut atlas_texture,
            )
            .unwrap();
        let texture_2 = builder
            .add_texture(
                &mut layout,
                &make_filled_image(square, colors[2]),
                &mut atlas_texture,
            )
            .unwrap();

        let expected_rects = [
            URect::from_corners(UVec2::new(1, 1), UVec2::new(4, 4)),
            URect::from_corners(UVec2::new(5, 1), UVec2::new(8, 4)),
            // If we didn't pad the right of the texture atlas, there would be just enough space to
            // fit this in the first row, but since we do pad the right, this gets pushed to the
            // next row.
            URect::from_corners(UVec2::new(1, 5), UVec2::new(4, 8)),
        ];
        assert_eq!(layout.textures[texture_0], expected_rects[0]);
        assert_eq!(layout.textures[texture_1], expected_rects[1]);
        assert_eq!(layout.textures[texture_2], expected_rects[2]);

        assert!(rect_contains_value(
            &atlas_texture,
            expected_rects[0],
            colors[0]
        ));
        assert!(rect_contains_value(
            &atlas_texture,
            expected_rects[1],
            colors[1]
        ));
        assert!(rect_contains_value(
            &atlas_texture,
            expected_rects[2],
            colors[2]
        ));
    }
}
