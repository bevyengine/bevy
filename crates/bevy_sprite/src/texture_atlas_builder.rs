use bevy_asset::AssetId;
use bevy_math::{URect, UVec2};
use bevy_render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::{Image, TextureFormatPixelInfo},
};
use bevy_utils::tracing::{debug, error, warn};
use bevy_utils::HashMap;
use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, PackedLocation,
    RectToInsert, TargetBin,
};
use thiserror::Error;

use crate::TextureAtlasLayout;

#[derive(Debug, Error)]
pub enum TextureAtlasBuilderError {
    #[error("could not pack textures into an atlas within the given bounds")]
    NotEnoughSpace,
    #[error("added a texture with the wrong format in an atlas")]
    WrongFormat,
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

pub type TextureAtlasBuilderResult<T> = Result<T, TextureAtlasBuilderError>;

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
    ) {
        let rect_width = (packed_location.width() - padding.x) as usize;
        let rect_height = (packed_location.height() - padding.y) as usize;
        let rect_x = packed_location.x() as usize;
        let rect_y = packed_location.y() as usize;
        let atlas_width = atlas_texture.width() as usize;
        let format_size = atlas_texture.texture_descriptor.format.pixel_size();

        for (texture_y, bound_y) in (rect_y..rect_y + rect_height).enumerate() {
            let begin = (bound_y * atlas_width + rect_x) * format_size;
            let end = begin + rect_width * format_size;
            let texture_begin = texture_y * rect_width * format_size;
            let texture_end = texture_begin + rect_width * format_size;
            atlas_texture.data[begin..end]
                .copy_from_slice(&texture.data[texture_begin..texture_end]);
        }
    }

    fn copy_converted_texture(
        &self,
        atlas_texture: &mut Image,
        texture: &Image,
        packed_location: &PackedLocation,
    ) {
        if self.format == texture.texture_descriptor.format {
            Self::copy_texture_to_atlas(atlas_texture, texture, packed_location, self.padding);
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
            );
        } else {
            error!(
                "Error converting texture from '{:?}' to '{:?}', ignoring",
                texture.texture_descriptor.format, self.format
            );
        }
    }

    #[deprecated(
        since = "0.14.0",
        note = "TextureAtlasBuilder::finish() was not idiomatic. Use TextureAtlasBuilder::build() instead."
    )]
    pub fn finish(&mut self) -> Result<(TextureAtlasLayout, Image), TextureAtlasBuilderError> {
        self.build()
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
    /// # use bevy_sprite::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::*;
    /// # use bevy_render::prelude::*;
    ///
    /// fn my_system(mut commands: Commands, mut textures: ResMut<Assets<Image>>, mut layouts: ResMut<Assets<TextureAtlasLayout>>) {
    ///     // Declare your builder
    ///     let mut builder = TextureAtlasBuilder::default();
    ///     // Customize it
    ///     // ...
    ///     // Build your texture and the atlas layout
    ///     let (atlas_layout, texture) = builder.build().unwrap();
    ///     let texture = textures.add(texture);
    ///     let layout = layouts.add(atlas_layout);
    ///     // Spawn your sprite
    ///     commands.spawn((
    ///         SpriteBundle { texture, ..Default::default() },
    ///         TextureAtlas::from(layout),
    ///     ));
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// If there is not enough space in the atlas texture, an error will
    /// be returned. It is then recommended to make a larger sprite sheet.
    pub fn build(&mut self) -> Result<(TextureAtlasLayout, Image), TextureAtlasBuilderError> {
        let max_width = self.max_size.x;
        let max_height = self.max_size.y;

        let mut current_width = self.initial_size.x;
        let mut current_height = self.initial_size.y;
        let mut rect_placements = None;
        let mut atlas_texture = Image::default();
        let mut rects_to_place = GroupedRectsToPlace::<usize>::new();

        // Adds textures to rectangle group packer
        for (index, (_, texture)) in self.textures_to_place.iter().enumerate() {
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

        while rect_placements.is_none() {
            if current_width > max_width || current_height > max_height {
                break;
            }

            let last_attempt = current_height == max_height && current_width == max_width;

            let mut target_bins = std::collections::BTreeMap::new();
            target_bins.insert(0, TargetBin::new(current_width, current_height, 1));
            rect_placements = match pack_rects(
                &rects_to_place,
                &mut target_bins,
                &volume_heuristic,
                &contains_smallest_box,
            ) {
                Ok(rect_placements) => {
                    atlas_texture = Image::new(
                        Extent3d {
                            width: current_width,
                            height: current_height,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        vec![
                            0;
                            self.format.pixel_size() * (current_width * current_height) as usize
                        ],
                        self.format,
                        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                    );
                    Some(rect_placements)
                }
                Err(rectangle_pack::RectanglePackError::NotEnoughBinSpace) => {
                    current_height = (current_height * 2).clamp(0, max_height);
                    current_width = (current_width * 2).clamp(0, max_width);
                    None
                }
            };

            if last_attempt {
                break;
            }
        }

        let rect_placements = rect_placements.ok_or(TextureAtlasBuilderError::NotEnoughSpace)?;

        let mut texture_rects = Vec::with_capacity(rect_placements.packed_locations().len());
        let mut texture_ids = HashMap::default();
        // We iterate through the textures to place to respect the insertion order for the texture indices
        for (index, (image_id, texture)) in self.textures_to_place.iter().enumerate() {
            let (_, packed_location) = rect_placements.packed_locations().get(&index).unwrap();

            let min = UVec2::new(packed_location.x(), packed_location.y());
            let max =
                min + UVec2::new(packed_location.width(), packed_location.height()) - self.padding;
            if let Some(image_id) = image_id {
                texture_ids.insert(*image_id, index);
            }
            texture_rects.push(URect { min, max });
            if texture.texture_descriptor.format != self.format && !self.auto_format_conversion {
                warn!(
                    "Loading a texture of format '{:?}' in an atlas with format '{:?}'",
                    texture.texture_descriptor.format, self.format
                );
                return Err(TextureAtlasBuilderError::WrongFormat);
            }
            self.copy_converted_texture(&mut atlas_texture, texture, packed_location);
        }

        Ok((
            TextureAtlasLayout {
                size: atlas_texture.size(),
                textures: texture_rects,
                texture_handles: Some(texture_ids),
            },
            atlas_texture,
        ))
    }
}
