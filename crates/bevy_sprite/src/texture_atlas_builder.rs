use crate::{Rect, TextureAtlas};
use bevy_asset::{Assets, Handle};
use bevy_log::{debug, error, warn};
use bevy_math::Vec2;
use bevy_render::texture::{Extent3d, Texture, TextureDimension, TextureFormat};
use bevy_utils::HashMap;
use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, PackedLocation,
    RectToInsert, TargetBin,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TextureAtlasBuilderError {
    #[error("could not pack textures into an atlas within the given bounds")]
    NotEnoughSpace,
    #[error("added a texture with the wrong format in an atlas")]
    WrongFormat,
}

#[derive(Debug)]
/// A builder which is used to create a texture atlas from many individual
/// sprites.
pub struct TextureAtlasBuilder {
    /// The grouped rects which must be placed with a key value pair of a
    /// texture handle to an index.
    rects_to_place: GroupedRectsToPlace<Handle<Texture>>,
    /// The initial atlas size in pixels.
    initial_size: Vec2,
    /// The absolute maximum size of the texture atlas in pixels.
    max_size: Vec2,
    /// The texture format for the textures that will be loaded in the atlas.
    format: TextureFormat,
    /// Enable automatic format conversion for textures if they are not in the atlas format.
    auto_format_conversion: bool,
}

impl Default for TextureAtlasBuilder {
    fn default() -> Self {
        Self {
            rects_to_place: GroupedRectsToPlace::new(),
            initial_size: Vec2::new(256., 256.),
            max_size: Vec2::new(2048., 2048.),
            format: TextureFormat::Rgba8UnormSrgb,
            auto_format_conversion: true,
        }
    }
}

pub type TextureAtlasBuilderResult<T> = Result<T, TextureAtlasBuilderError>;

impl TextureAtlasBuilder {
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

    /// Sets the texture format for textures in the atlas.
    pub fn format(mut self, format: TextureFormat) -> Self {
        self.format = format;
        self
    }

    /// Control whether the added texture should be converted to the atlas format, if different.
    pub fn auto_format_conversion(mut self, auto_format_conversion: bool) -> Self {
        self.auto_format_conversion = auto_format_conversion;
        self
    }

    /// Adds a texture to be copied to the texture atlas.
    pub fn add_texture(&mut self, texture_handle: Handle<Texture>, texture: &Texture) {
        self.rects_to_place.push_rect(
            texture_handle,
            None,
            RectToInsert::new(texture.size.width, texture.size.height, 1),
        )
    }

    fn copy_texture_to_atlas(
        atlas_texture: &mut Texture,
        texture: &Texture,
        packed_location: &PackedLocation,
    ) {
        let rect_width = packed_location.width() as usize;
        let rect_height = packed_location.height() as usize;
        let rect_x = packed_location.x() as usize;
        let rect_y = packed_location.y() as usize;
        let atlas_width = atlas_texture.size.width as usize;
        let format_size = atlas_texture.format.pixel_size();

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
        atlas_texture: &mut Texture,
        texture: &Texture,
        packed_location: &PackedLocation,
    ) {
        if self.format == texture.format {
            Self::copy_texture_to_atlas(atlas_texture, texture, packed_location);
        } else if let Some(converted_texture) = texture.convert(self.format) {
            debug!(
                "Converting texture from '{:?}' to '{:?}'",
                texture.format, self.format
            );
            Self::copy_texture_to_atlas(atlas_texture, &converted_texture, packed_location);
        } else {
            error!(
                "Error converting texture from '{:?}' to '{:?}', ignoring",
                texture.format, self.format
            );
        }
    }

    /// Consumes the builder and returns a result with a new texture atlas.
    ///
    /// Internally it copies all rectangles from the textures and copies them
    /// into a new texture which the texture atlas will use. It is not useful to
    /// hold a strong handle to the texture afterwards else it will exist twice
    /// in memory.
    ///
    /// # Errors
    ///
    /// If there is not enough space in the atlas texture, an error will
    /// be returned. It is then recommended to make a larger sprite sheet.
    pub fn finish(
        self,
        textures: &mut Assets<Texture>,
    ) -> Result<TextureAtlas, TextureAtlasBuilderError> {
        let initial_width = self.initial_size.x as u32;
        let initial_height = self.initial_size.y as u32;
        let max_width = self.max_size.x as u32;
        let max_height = self.max_size.y as u32;

        let mut current_width = initial_width;
        let mut current_height = initial_height;
        let mut rect_placements = None;
        let mut atlas_texture = Texture::default();

        while rect_placements.is_none() {
            if current_width > max_width || current_height > max_height {
                break;
            }

            let last_attempt = current_height == max_height && current_width == max_width;

            let mut target_bins = std::collections::BTreeMap::new();
            target_bins.insert(0, TargetBin::new(current_width, current_height, 1));
            rect_placements = match pack_rects(
                &self.rects_to_place,
                &mut target_bins,
                &volume_heuristic,
                &contains_smallest_box,
            ) {
                Ok(rect_placements) => {
                    atlas_texture = Texture::new_fill(
                        Extent3d::new(current_width, current_height, 1),
                        TextureDimension::D2,
                        &[0, 0, 0, 0],
                        self.format,
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
        let mut texture_handles = HashMap::default();
        for (texture_handle, (_, packed_location)) in rect_placements.packed_locations().iter() {
            let texture = textures.get(texture_handle).unwrap();
            let min = Vec2::new(packed_location.x() as f32, packed_location.y() as f32);
            let max = min
                + Vec2::new(
                    packed_location.width() as f32,
                    packed_location.height() as f32,
                );
            texture_handles.insert(texture_handle.clone_weak(), texture_rects.len());
            texture_rects.push(Rect { min, max });
            if texture.format != self.format && !self.auto_format_conversion {
                warn!(
                    "Loading a texture of format '{:?}' in an atlas with format '{:?}'",
                    texture.format, self.format
                );
                return Err(TextureAtlasBuilderError::WrongFormat);
            }
            self.copy_converted_texture(&mut atlas_texture, texture, packed_location);
        }
        Ok(TextureAtlas {
            size: atlas_texture.size.as_vec3().truncate(),
            texture: textures.add(atlas_texture),
            textures: texture_rects,
            texture_handles: Some(texture_handles),
        })
    }
}
