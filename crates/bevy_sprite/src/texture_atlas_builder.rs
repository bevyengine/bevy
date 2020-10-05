use crate::{Rect, TextureAtlas};
use bevy_asset::{Assets, Handle};
use bevy_math::Vec2;
use bevy_render::texture::{Texture, TextureFormat};
use bevy_utils::HashMap;
use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, PackedLocation,
    RectToInsert, TargetBin,
};
use thiserror::Error;

#[derive(Debug)]
pub struct TextureAtlasBuilder {
    pub textures: Vec<Handle<Texture>>,
    pub rects_to_place: GroupedRectsToPlace<Handle<Texture>>,
    pub initial_size: Vec2,
    pub max_size: Vec2,
}

impl Default for TextureAtlasBuilder {
    fn default() -> Self {
        Self::new(Vec2::new(256., 256.), Vec2::new(2048., 2048.))
    }
}

#[derive(Debug, Error)]
pub enum RectanglePackError {
    #[error("Could not pack textures into an atlas within the given bounds")]
    NotEnoughSpace,
}

impl TextureAtlasBuilder {
    pub fn new(initial_size: Vec2, max_size: Vec2) -> Self {
        Self {
            textures: Default::default(),
            rects_to_place: GroupedRectsToPlace::new(),
            initial_size,
            max_size,
        }
    }

    pub fn add_texture(&mut self, texture_handle: Handle<Texture>, texture: &Texture) {
        self.rects_to_place.push_rect(
            texture_handle,
            None,
            RectToInsert::new(texture.size.x() as u32, texture.size.y() as u32, 1),
        )
    }

    fn place_texture(
        &mut self,
        atlas_texture: &mut Texture,
        texture: &Texture,
        packed_location: &PackedLocation,
    ) {
        let rect_width = packed_location.width() as usize;
        let rect_height = packed_location.height() as usize;
        let rect_x = packed_location.x() as usize;
        let rect_y = packed_location.y() as usize;
        let atlas_width = atlas_texture.size.x() as usize;
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

    pub fn finish(
        mut self,
        textures: &mut Assets<Texture>,
    ) -> Result<TextureAtlas, RectanglePackError> {
        let initial_width = self.initial_size.x() as u32;
        let initial_height = self.initial_size.y() as u32;
        let max_width = self.max_size.x() as u32;
        let max_height = self.max_size.y() as u32;

        let mut current_width = initial_width;
        let mut current_height = initial_height;
        let mut rect_placements = None;
        let mut atlas_texture = Texture::default();

        while rect_placements.is_none() {
            if current_width > max_width || current_height > max_height {
                rect_placements = None;
                break;
            }
            let mut target_bins = std::collections::HashMap::new();
            target_bins.insert(0, TargetBin::new(current_width, current_height, 1));
            atlas_texture = Texture::new_fill(
                Vec2::new(current_width as f32, current_height as f32),
                &[0, 0, 0, 0],
                TextureFormat::Rgba8UnormSrgb,
            );
            rect_placements = match pack_rects(
                &self.rects_to_place,
                target_bins,
                &volume_heuristic,
                &contains_smallest_box,
            ) {
                Ok(rect_placements) => Some(rect_placements),
                Err(rectangle_pack::RectanglePackError::NotEnoughBinSpace) => {
                    current_width *= 2;
                    current_height *= 2;
                    None
                }
            }
        }

        let rect_placements = rect_placements.ok_or(RectanglePackError::NotEnoughSpace)?;

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
            texture_handles.insert(*texture_handle, texture_rects.len());
            texture_rects.push(Rect { min, max });
            self.place_texture(&mut atlas_texture, texture, packed_location);
        }
        Ok(TextureAtlas {
            size: atlas_texture.size,
            texture: textures.add(atlas_texture),
            textures: texture_rects,
            texture_handles: Some(texture_handles),
        })
    }
}
