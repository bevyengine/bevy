use bevy_asset::{Assets, Handle};
use bevy_log::{debug, error, warn};
use bevy_math::Vec2;
use bevy_render2::{
    image::{Image, TextureFormatPixelInfo},
    render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_utils::HashMap;
use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, PackedLocation,
    RectToInsert, TargetBin,
};
use thiserror::Error;

use crate::{image_atlas::ImageAtlas, Rect};

#[derive(Debug, Error)]
pub enum ImageAtlasBuilderError {
    #[error("could not pack the images into the atlas within the given bounds")]
    NotEnoughSpace,
    #[error("added a image with the wrong format to the atlas")]
    WrongFormat,
}

#[derive(Debug)]
/// A builder which is used to create an [`ImageAtlas`] from many individual images.
pub struct ImageAtlasBuilder {
    /// The regions which must be placed with a key value pair of a image handle and index.
    regions_to_place: GroupedRectsToPlace<Handle<Image>>,
    /// The initial atlas size in pixels.
    initial_size: Vec2,
    /// The absolute maximum size of the atlas in pixels.
    max_size: Vec2,
    /// The texture format for the images that will be loaded in the atlas.
    format: TextureFormat,
    /// Enable automatic format conversion for images that have a different format than the atlas.
    auto_format_conversion: bool,
}

impl Default for ImageAtlasBuilder {
    fn default() -> Self {
        Self {
            regions_to_place: GroupedRectsToPlace::new(),
            initial_size: Vec2::new(256., 256.),
            max_size: Vec2::new(2048., 2048.),
            format: TextureFormat::Rgba8UnormSrgb,
            auto_format_conversion: true,
        }
    }
}

impl ImageAtlasBuilder {
    /// Sets the initial size of the atlas in pixels.
    pub fn initial_size(mut self, size: Vec2) -> Self {
        self.initial_size = size;
        self
    }

    /// Sets the max size of the atlas in pixels.
    pub fn max_size(mut self, size: Vec2) -> Self {
        self.max_size = size;
        self
    }

    /// Sets the texture format for images of the atlas.
    pub fn format(mut self, format: TextureFormat) -> Self {
        self.format = format;
        self
    }

    /// Control whether the added images should be converted to the format of the atlas,
    /// if they are different.
    pub fn auto_format_conversion(mut self, auto_format_conversion: bool) -> Self {
        self.auto_format_conversion = auto_format_conversion;
        self
    }

    // Todo: change this to only take in the size of the image
    /// Adds a image to be copied to the atlas.
    pub fn add_image(&mut self, image_handle: Handle<Image>, image: &Image) {
        self.regions_to_place.push_rect(
            image_handle,
            None,
            RectToInsert::new(
                image.texture_descriptor.size.width,
                image.texture_descriptor.size.height,
                1,
            ),
        )
    }

    /// Copies the `image` data to the region in the `atlas_image`.
    fn copy_image_to_atlas(atlas_image: &mut Image, image: &Image, region: &PackedLocation) {
        let region_x = region.x() as usize;
        let region_y = region.y() as usize;
        let region_width = region.width() as usize;
        let region_height = region.height() as usize;
        let atlas_width = atlas_image.texture_descriptor.size.width as usize;
        let pixel_size = atlas_image.texture_descriptor.format.pixel_size();

        // copy row by row
        for (image_row, bound_y) in (region_y..region_y + region_height).enumerate() {
            let atlas_begin = (bound_y * atlas_width + region_x) * pixel_size;
            let atlas_end = atlas_begin + region_width * pixel_size;
            let image_begin = image_row * region_width * pixel_size;
            let image_end = image_begin + region_width * pixel_size;
            atlas_image.data[atlas_begin..atlas_end]
                .copy_from_slice(&image.data[image_begin..image_end]);
        }
    }

    // Todo: check auto_format_conversion here and fail explicitly with result if format can not be converted
    /// Copies the `image` data to the region in the `atlas_image` and tries to convert the format
    /// if they are different.
    fn copy_image_converting(
        &self,
        atlas_image: &mut Image,
        image: &Image,
        region: &PackedLocation,
    ) {
        if self.format == image.texture_descriptor.format {
            Self::copy_image_to_atlas(atlas_image, image, region);
        } else if let Some(converted_image) = image.convert(self.format) {
            debug!(
                "Converting image from '{:?}' to '{:?}'",
                image.texture_descriptor.format, self.format
            );
            Self::copy_image_to_atlas(atlas_image, &converted_image, region);
        } else {
            error!(
                "Error converting image from '{:?}' to '{:?}', ignoring",
                image.texture_descriptor.format, self.format
            );
        }
    }

    /// Consumes the builder and returns a result with a new [`ImageAtlas`].
    ///
    /// Internally it copies all images into the regions of a new image which the atlas will use.
    /// It is not useful to hold strong handles to the copied images afterwards else they will
    /// exist twice in memory.
    ///
    /// # Errors
    ///
    /// If there is not enough space in the `source_image` of the atlas, an error will
    /// be returned. It is then recommended to make a larger atlas.
    pub fn finish(self, images: &mut Assets<Image>) -> Result<ImageAtlas, ImageAtlasBuilderError> {
        let initial_width = self.initial_size.x as u32;
        let initial_height = self.initial_size.y as u32;
        let max_width = self.max_size.x as u32;
        let max_height = self.max_size.y as u32;

        let mut current_width = initial_width;
        let mut current_height = initial_height;
        let mut region_placements = None;
        let mut atlas_image = Image::default();

        while region_placements.is_none() {
            if current_width > max_width || current_height > max_height {
                break;
            }

            let last_attempt = current_height == max_height && current_width == max_width;

            let mut target_bins = std::collections::BTreeMap::new();
            target_bins.insert(0, TargetBin::new(current_width, current_height, 1));
            region_placements = match pack_rects(
                &self.regions_to_place,
                &mut target_bins,
                &volume_heuristic,
                &contains_smallest_box,
            ) {
                Ok(region_placements) => {
                    atlas_image = Image::new_fill(
                        Extent3d {
                            width: current_width,
                            height: current_height,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        &[0, 0, 0, 0],
                        self.format,
                    );
                    Some(region_placements)
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

        let region_placements = region_placements.ok_or(ImageAtlasBuilderError::NotEnoughSpace)?;

        let mut regions = Vec::with_capacity(region_placements.packed_locations().len());
        let mut image_handles = HashMap::default();
        for (image_handle, (_, region)) in region_placements.packed_locations().iter() {
            let image = images.get(image_handle).unwrap();
            let min = Vec2::new(region.x() as f32, region.y() as f32);
            let max = min + Vec2::new(region.width() as f32, region.height() as f32);
            image_handles.insert(image_handle.clone_weak(), regions.len());
            regions.push(Rect { min, max });
            if image.texture_descriptor.format != self.format && !self.auto_format_conversion {
                warn!(
                    "Loading a image of format '{:?}' in an atlas with format '{:?}'",
                    image.texture_descriptor.format, self.format
                );
                return Err(ImageAtlasBuilderError::WrongFormat);
            }
            self.copy_image_converting(&mut atlas_image, image, region);
        }
        Ok(ImageAtlas {
            size: Vec2::new(
                atlas_image.texture_descriptor.size.width as f32,
                atlas_image.texture_descriptor.size.height as f32,
            ),
            source_image: images.add(atlas_image),
            regions,
            image_handles: Some(image_handles),
        })
    }
}
