use bevy_asset::{AssetId, RenderAssetUsages};
use bevy_math::{URect, UVec2};
use bevy_platform::collections::HashMap;
use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, PackedLocation,
    RectToInsert, RectanglePackOk, TargetBin,
};
use thiserror::Error;
use tracing::{debug, error, warn};
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

use crate::{Image, TextureAccessError, TextureFormatPixelInfo};
use crate::{TextureAtlasLayout, TextureAtlasSources};

/// Errors returned by [`TextureAtlasBuilder`].
#[derive(Debug, Error)]
pub enum TextureAtlasBuilderError {
    /// The atlas texture wasn't large enough to fit the texture
    #[error("could not pack textures into an atlas within the given bounds")]
    NotEnoughSpace,
    /// Attempted to add a texture with a different format
    #[error("added a texture with the wrong format in an atlas")]
    WrongFormat,
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

#[derive(Debug)]
#[must_use]
/// A builder which is used to create a texture atlas from many individual
/// sprites.
pub struct TextureAtlasBuilder<'a> {
    /// Collection of texture's asset id (optional) and image data to be packed into an atlas
    textures_to_place: Vec<(Option<AssetId<Image>>, &'a Image)>,
    /// The initial atlas size in pixels.
    initial_size: UVec2,
    /// The absolute maximum size of the texture atlas in pixels.
    max_size: UVec2,
    /// The texture format for the textures that will be loaded in the atlas.
    format: TextureFormat,
    /// Enable automatic format conversion for textures if they are not in the atlas format.
    auto_format_conversion: bool,
    /// The amount of padding in pixels to add along the right and bottom edges of the texture rects.
    padding: UVec2,
}

impl Default for TextureAtlasBuilder<'_> {
    fn default() -> Self {
        Self {
            textures_to_place: Vec::new(),
            initial_size: UVec2::splat(256),
            max_size: UVec2::splat(2048),
            format: TextureFormat::Rgba8UnormSrgb,
            auto_format_conversion: true,
            padding: UVec2::ZERO,
        }
    }
}

/// The [`Result`] type used by [`TextureAtlasBuilder`].
pub type TextureAtlasBuilderResult<T> = Result<T, TextureAtlasBuilderError>;

/// A texture added to a [`TextureAtlasBuilder`].
#[derive(Debug, Clone, Copy)]
pub struct UnplacedAtlasTexture<'a> {
    /// The optional asset id for the texture.
    pub image_id: Option<AssetId<Image>>,
    /// The source image to add to the texture atlas.
    pub texture: &'a Image,
}

/// The result of building a texture atlas while allowing unplaced textures.
#[derive(Debug)]
pub struct TextureAtlasPartialBuildResult<'a> {
    /// The atlas layout for the successfully placed textures.
    pub layout: TextureAtlasLayout,
    /// Sources for the successfully placed textures.
    pub sources: TextureAtlasSources,
    /// The atlas texture containing the successfully placed textures.
    pub texture: Image,
    /// Textures that could not be placed into the atlas.
    pub unplaced_textures: Vec<UnplacedAtlasTexture<'a>>,
}

impl<'a> TextureAtlasBuilder<'a> {
    /// Sets the initial size of the atlas in pixels.
    pub fn initial_size(&mut self, size: UVec2) -> &mut Self {
        self.initial_size = size;
        self
    }

    /// Sets the max size of the atlas in pixels.
    pub fn max_size(&mut self, size: UVec2) -> &mut Self {
        self.max_size = size;
        self
    }

    /// Sets the texture format for textures in the atlas.
    pub fn format(&mut self, format: TextureFormat) -> &mut Self {
        self.format = format;
        self
    }

    /// Control whether the added texture should be converted to the atlas format, if different.
    pub fn auto_format_conversion(&mut self, auto_format_conversion: bool) -> &mut Self {
        self.auto_format_conversion = auto_format_conversion;
        self
    }

    /// Adds a texture to be copied to the texture atlas.
    ///
    /// Optionally an asset id can be passed that can later be used with the texture layout to retrieve the index of this texture.
    /// The insertion order will reflect the index of the added texture in the finished texture atlas.
    pub fn add_texture(
        &mut self,
        image_id: Option<AssetId<Image>>,
        texture: &'a Image,
    ) -> &mut Self {
        self.textures_to_place.push((image_id, texture));
        self
    }

    /// Sets the amount of padding in pixels to add between the textures in the texture atlas.
    ///
    /// The `x` value provide will be added to the right edge, while the `y` value will be added to the bottom edge.
    pub fn padding(&mut self, padding: UVec2) -> &mut Self {
        self.padding = padding;
        self
    }

    fn copy_texture_to_atlas(
        atlas_texture: &mut Image,
        texture: &Image,
        packed_location: &PackedLocation,
        padding: UVec2,
    ) -> TextureAtlasBuilderResult<()> {
        let rect_width = (packed_location.width() - padding.x) as usize;
        let rect_height = (packed_location.height() - padding.y) as usize;
        let rect_x = packed_location.x() as usize;
        let rect_y = packed_location.y() as usize;
        let atlas_width = atlas_texture.width() as usize;
        let format_size = atlas_texture.texture_descriptor.format.pixel_size()?;

        let Some(ref mut atlas_data) = atlas_texture.data else {
            return Err(TextureAtlasBuilderError::UninitializedAtlas);
        };
        let Some(ref data) = texture.data else {
            return Err(TextureAtlasBuilderError::UninitializedSourceTexture);
        };
        for (texture_y, bound_y) in (rect_y..rect_y + rect_height).enumerate() {
            let begin = (bound_y * atlas_width + rect_x) * format_size;
            let end = begin + rect_width * format_size;
            let texture_begin = texture_y * rect_width * format_size;
            let texture_end = texture_begin + rect_width * format_size;
            atlas_data[begin..end].copy_from_slice(&data[texture_begin..texture_end]);
        }
        Ok(())
    }

    fn copy_converted_texture(
        &self,
        atlas_texture: &mut Image,
        texture: &Image,
        packed_location: &PackedLocation,
    ) -> TextureAtlasBuilderResult<()> {
        if self.format == texture.texture_descriptor.format {
            Self::copy_texture_to_atlas(atlas_texture, texture, packed_location, self.padding)?;
        } else if let Some(converted_texture) = texture.convert(self.format) {
            debug!(
                "Converting texture from '{:?}' to '{:?}'",
                texture.texture_descriptor.format, self.format
            );
            Self::copy_texture_to_atlas(
                atlas_texture,
                &converted_texture,
                packed_location,
                self.padding,
            )?;
        } else {
            error!(
                "Error converting texture from '{:?}' to '{:?}', ignoring",
                texture.texture_descriptor.format, self.format
            );
        }
        Ok(())
    }

    fn create_rects_to_place(&self, texture_indices: &[usize]) -> GroupedRectsToPlace<usize> {
        let mut rects_to_place = GroupedRectsToPlace::<usize>::new();

        for &index in texture_indices {
            let (_, texture) = self.textures_to_place[index];
            rects_to_place.push_rect(
                index,
                None,
                RectToInsert::new(
                    texture.width() + self.padding.x,
                    texture.height() + self.padding.y,
                    1,
                ),
            );
        }

        rects_to_place
    }

    fn pack_textures(
        &self,
        texture_indices: &[usize],
    ) -> Option<(RectanglePackOk<usize, i32>, UVec2)> {
        let max_width = self.max_size.x;
        let max_height = self.max_size.y;
        let mut current_width = self.initial_size.x;
        let mut current_height = self.initial_size.y;
        let rects_to_place = self.create_rects_to_place(texture_indices);

        loop {
            if current_width > max_width || current_height > max_height {
                return None;
            }

            let last_attempt = current_height == max_height && current_width == max_width;

            let mut target_bins = alloc::collections::BTreeMap::new();
            target_bins.insert(0, TargetBin::new(current_width, current_height, 1));

            match pack_rects(
                &rects_to_place,
                &mut target_bins,
                &volume_heuristic,
                &contains_smallest_box,
            ) {
                Ok(rect_placements) => {
                    return Some((rect_placements, UVec2::new(current_width, current_height)));
                }
                Err(rectangle_pack::RectanglePackError::NotEnoughBinSpace) if last_attempt => {
                    return None;
                }
                Err(rectangle_pack::RectanglePackError::NotEnoughBinSpace) => {
                    current_height = (current_height * 2).clamp(0, max_height);
                    current_width = (current_width * 2).clamp(0, max_width);
                }
            }
        }
    }

    fn create_atlas_texture(&self, size: UVec2) -> TextureAtlasBuilderResult<Image> {
        Ok(Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![0; self.format.pixel_size()? * (size.x * size.y) as usize],
            self.format,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ))
    }

    fn build_texture_atlas_from_placements<BinId: core::hash::Hash + Eq + PartialEq>(
        &self,
        texture_indices: &[usize],
        rect_placements: &RectanglePackOk<usize, BinId>,
        mut atlas_texture: Image,
    ) -> TextureAtlasBuilderResult<(TextureAtlasLayout, TextureAtlasSources, Image)> {
        let mut texture_rects = Vec::with_capacity(texture_indices.len());
        let mut texture_ids = <HashMap<_, _>>::default();

        // We iterate through the textures to place to respect the insertion order for the texture indices
        for (atlas_index, &source_index) in texture_indices.iter().enumerate() {
            let (image_id, texture) = self.textures_to_place[source_index];
            let (_, packed_location) = rect_placements
                .packed_locations()
                .get(&source_index)
                .unwrap();

            let min = UVec2::new(packed_location.x(), packed_location.y());
            let max =
                min + UVec2::new(packed_location.width(), packed_location.height()) - self.padding;
            if let Some(image_id) = image_id {
                texture_ids.insert(image_id, atlas_index);
            }
            texture_rects.push(URect { min, max });
            if texture.texture_descriptor.format != self.format && !self.auto_format_conversion {
                warn!(
                    "Loading a texture of format '{:?}' in an atlas with format '{:?}'",
                    texture.texture_descriptor.format, self.format
                );
                return Err(TextureAtlasBuilderError::WrongFormat);
            }
            self.copy_converted_texture(&mut atlas_texture, texture, packed_location)?;
        }

        Ok((
            TextureAtlasLayout {
                size: atlas_texture.size(),
                textures: texture_rects,
            },
            TextureAtlasSources { texture_ids },
            atlas_texture,
        ))
    }

    /// Consumes the builder, and returns the newly created texture atlas and
    /// the associated atlas layout.
    ///
    /// Assigns indices to the textures based on the insertion order.
    /// Internally it copies all rectangles from the textures and copies them
    /// into a new texture.
    ///
    /// # Usage
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::*;
    /// # use bevy_image::prelude::*;
    ///
    /// fn my_system(mut textures: ResMut<Assets<Image>>, mut layouts: ResMut<Assets<TextureAtlasLayout>>) {
    ///     // Declare your builder
    ///     let mut builder = TextureAtlasBuilder::default();
    ///     // Customize it
    ///     // ...
    ///     // Build your texture and the atlas layout
    ///     let (atlas_layout, atlas_sources, texture) = builder.build().unwrap();
    ///     let texture = textures.add(texture);
    ///     let layout = layouts.add(atlas_layout);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// If there is not enough space in the atlas texture, an error will
    /// be returned. It is then recommended to make a larger sprite sheet.
    pub fn build(
        &mut self,
    ) -> TextureAtlasBuilderResult<(TextureAtlasLayout, TextureAtlasSources, Image)> {
        let texture_indices = (0..self.textures_to_place.len()).collect::<Vec<_>>();
        let (rect_placements, atlas_size) = self
            .pack_textures(&texture_indices)
            .ok_or(TextureAtlasBuilderError::NotEnoughSpace)?;
        let atlas_texture = self.create_atlas_texture(atlas_size)?;
        self.build_texture_atlas_from_placements(&texture_indices, &rect_placements, atlas_texture)
    }

    /// Consumes the builder and returns a texture atlas along with any textures that could not be
    /// placed.
    ///
    /// If all textures fit, this behaves like `build()`.
    /// Otherwise, textures are greedily selected in insertion order.
    /// Errors unrelated to available space are still returned as hard errors.
    pub fn build_partial(
        &mut self,
    ) -> TextureAtlasBuilderResult<TextureAtlasPartialBuildResult<'a>> {
        let texture_indices = (0..self.textures_to_place.len()).collect::<Vec<_>>();

        if let Some((rect_placements, atlas_size)) = self.pack_textures(&texture_indices) {
            let atlas_texture = self.create_atlas_texture(atlas_size)?;
            let (layout, sources, texture) = self.build_texture_atlas_from_placements(
                &texture_indices,
                &rect_placements,
                atlas_texture,
            )?;
            return Ok(TextureAtlasPartialBuildResult {
                layout,
                sources,
                texture,
                unplaced_textures: Vec::new(),
            });
        }

        let mut placed_texture_indices = Vec::new();
        let mut unplaced_textures = Vec::new();

        for index in texture_indices {
            let mut candidate_indices = placed_texture_indices.clone();
            candidate_indices.push(index);

            if self.pack_textures(&candidate_indices).is_some() {
                placed_texture_indices.push(index);
            } else {
                let (image_id, texture) = self.textures_to_place[index];
                unplaced_textures.push(UnplacedAtlasTexture { image_id, texture });
            }
        }

        if placed_texture_indices.is_empty() {
            return Err(TextureAtlasBuilderError::NotEnoughSpace);
        }

        let (rect_placements, atlas_size) = self
            .pack_textures(&placed_texture_indices)
            .ok_or(TextureAtlasBuilderError::NotEnoughSpace)?;
        let atlas_texture = self.create_atlas_texture(atlas_size)?;
        let (layout, sources, texture) = self.build_texture_atlas_from_placements(
            &placed_texture_indices,
            &rect_placements,
            atlas_texture,
        )?;

        Ok(TextureAtlasPartialBuildResult {
            layout,
            sources,
            texture,
            unplaced_textures,
        })
    }
}

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;

    use bevy_asset::{AssetId, AssetIndex, RenderAssetUsages};
    use bevy_math::{URect, UVec2};
    use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

    use crate::{Image, TextureAtlasBuilder, TextureAtlasBuilderError};

    fn make_filled_image(size: UVec2, pixel_rgba_bytes: [u8; 4]) -> Image {
        Image::new_fill(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &pixel_rgba_bytes,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::all(),
        )
    }

    fn make_image_id(index: u64) -> AssetId<Image> {
        AssetId::Index {
            index: AssetIndex::from_bits(index),
            marker: PhantomData,
        }
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
    fn build_partial_returns_unplaced_textures() {
        let mut builder = TextureAtlasBuilder::default();
        builder
            .initial_size(UVec2::new(16, 8))
            .max_size(UVec2::new(16, 8))
            .format(TextureFormat::Rgba8Unorm);

        let colors = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]];
        let textures = [
            make_filled_image(UVec2::new(8, 8), colors[0]),
            make_filled_image(UVec2::new(8, 8), colors[1]),
            make_filled_image(UVec2::new(8, 8), colors[2]),
        ];
        let ids = [make_image_id(1), make_image_id(2), make_image_id(3)];

        for (id, texture) in ids.into_iter().zip(textures.iter()) {
            builder.add_texture(Some(id), texture);
        }

        let result = builder.build_partial().unwrap();

        assert_eq!(result.layout.len(), 2);
        assert_eq!(result.unplaced_textures.len(), 1);
        assert_eq!(result.unplaced_textures[0].image_id, Some(make_image_id(3)));
        assert_eq!(result.sources.texture_index(make_image_id(1)), Some(0));
        assert_eq!(result.sources.texture_index(make_image_id(2)), Some(1));
        assert_eq!(result.sources.texture_index(make_image_id(3)), None);

        let first_rect = result
            .sources
            .texture_rect(&result.layout, make_image_id(1))
            .unwrap();
        let second_rect = result
            .sources
            .texture_rect(&result.layout, make_image_id(2))
            .unwrap();
        assert!(rect_contains_value(&result.texture, first_rect, colors[0]));
        assert!(rect_contains_value(&result.texture, second_rect, colors[1]));
    }

    #[test]
    fn build_partial_errors_when_nothing_fits() {
        let texture = make_filled_image(UVec2::new(16, 16), [255, 0, 0, 255]);

        let mut builder = TextureAtlasBuilder::default();
        builder
            .initial_size(UVec2::new(8, 8))
            .max_size(UVec2::new(8, 8))
            .format(TextureFormat::Rgba8Unorm)
            .add_texture(None, &texture);

        assert!(matches!(
            builder.build_partial(),
            Err(TextureAtlasBuilderError::NotEnoughSpace)
        ));
    }

    #[test]
    fn build_partial_has_no_unplaced_textures_when_everything_fits() {
        let mut builder = TextureAtlasBuilder::default();
        builder
            .initial_size(UVec2::new(16, 8))
            .max_size(UVec2::new(16, 8))
            .format(TextureFormat::Rgba8Unorm);

        let textures = [
            make_filled_image(UVec2::new(8, 8), [255, 0, 0, 255]),
            make_filled_image(UVec2::new(8, 8), [0, 255, 0, 255]),
        ];

        for texture in &textures {
            builder.add_texture(None, texture);
        }

        let result = builder.build_partial().unwrap();

        assert!(result.unplaced_textures.is_empty());
        assert_eq!(result.layout.len(), 2);
    }

    #[test]
    fn build_still_returns_not_enough_space_when_all_textures_do_not_fit() {
        let mut builder = TextureAtlasBuilder::default();
        builder
            .initial_size(UVec2::new(16, 8))
            .max_size(UVec2::new(16, 8))
            .format(TextureFormat::Rgba8Unorm);

        let textures = [
            make_filled_image(UVec2::new(8, 8), [255, 0, 0, 255]),
            make_filled_image(UVec2::new(8, 8), [0, 255, 0, 255]),
            make_filled_image(UVec2::new(8, 8), [0, 0, 255, 255]),
        ];

        for texture in &textures {
            builder.add_texture(None, texture);
        }

        assert!(matches!(
            builder.build(),
            Err(TextureAtlasBuilderError::NotEnoughSpace)
        ));
    }

    #[test]
    fn build_partial_reindexes_texture_ids_to_match_atlas_indices() {
        let mut builder = TextureAtlasBuilder::default();
        builder
            .initial_size(UVec2::new(16, 8))
            .max_size(UVec2::new(16, 8))
            .format(TextureFormat::Rgba8Unorm);

        let textures = [
            make_filled_image(UVec2::new(8, 8), [255, 0, 0, 255]),
            make_filled_image(UVec2::new(8, 8), [0, 255, 0, 255]),
            make_filled_image(UVec2::new(8, 8), [0, 0, 255, 255]),
        ];
        let ids = [make_image_id(11), make_image_id(12), make_image_id(13)];

        for (id, texture) in ids.into_iter().zip(textures.iter()) {
            builder.add_texture(Some(id), texture);
        }

        let result = builder.build_partial().unwrap();

        assert_eq!(result.sources.texture_index(make_image_id(11)), Some(0));
        assert_eq!(result.sources.texture_index(make_image_id(12)), Some(1));
        assert_eq!(result.sources.texture_index(make_image_id(13)), None);
    }

    #[test]
    fn build_partial_returns_wrong_format_as_a_hard_error() {
        let texture = Image::new_fill(
            Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );

        let mut builder = TextureAtlasBuilder::default();
        builder
            .initial_size(UVec2::new(8, 8))
            .max_size(UVec2::new(8, 8))
            .format(TextureFormat::Rgba8Unorm)
            .auto_format_conversion(false)
            .add_texture(None, &texture);

        assert!(matches!(
            builder.build_partial(),
            Err(TextureAtlasBuilderError::WrongFormat)
        ));
    }
}
