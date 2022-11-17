use std::collections::VecDeque;

use crate::{error::TextError, Font, FontAtlas, TextSettings};
use ab_glyph::{GlyphId, OutlinedGlyph, Point};
use bevy_asset::{Assets, Handle};
use bevy_math::Vec2;
use bevy_reflect::TypeUuid;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_utils::FloatOrd;
use bevy_utils::HashMap;

type FontSizeKey = FloatOrd;

#[derive(TypeUuid)]
#[uuid = "73ba778b-b6b5-4f45-982d-d21b6b86ace2"]
pub struct FontAtlasSet {
    font_atlases: HashMap<FontSizeKey, Vec<FontAtlas>>,
    queue: VecDeque<FontSizeKey>,
}

#[derive(Debug, Clone)]
pub struct GlyphAtlasInfo {
    pub texture_atlas: Handle<TextureAtlas>,
    pub glyph_index: usize,
}

impl Default for FontAtlasSet {
    fn default() -> Self {
        FontAtlasSet {
            font_atlases: HashMap::with_capacity_and_hasher(1, Default::default()),
            queue: VecDeque::new(),
        }
    }
}

impl FontAtlasSet {
    pub fn iter(&self) -> impl Iterator<Item = (&FontSizeKey, &Vec<FontAtlas>)> {
        self.font_atlases.iter()
    }

    pub fn has_glyph(&self, glyph_id: GlyphId, glyph_position: Point, font_size: f32) -> bool {
        self.font_atlases
            .get(&FloatOrd(font_size))
            .map_or(false, |font_atlas| {
                font_atlas
                    .iter()
                    .any(|atlas| atlas.has_glyph(glyph_id, glyph_position.into()))
            })
    }

    pub fn add_glyph_to_atlas(
        &mut self,
        font_size: f32,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Image>,
        outlined_glyph: OutlinedGlyph,
        text_settings: &TextSettings,
    ) -> Result<GlyphAtlasInfo, TextError> {
        let glyph = outlined_glyph.glyph();
        let glyph_id = glyph.id;
        let glyph_position = glyph.position;
        let font_size_key = FloatOrd(font_size);

        self.update_last_used(&font_size_key);

        let mut len = self.font_atlases.len();

        let font_atlases = self.font_atlases.entry(font_size_key).or_insert_with(|| {
            len += 1;

            vec![FontAtlas::new(
                textures,
                texture_atlases,
                Vec2::splat(512.0),
            )]
        });

        if !text_settings.allow_dynamic_font_size && len > text_settings.max_font_atlases.get() {
            return Err(TextError::ExceedMaxTextAtlases(
                text_settings.max_font_atlases.get(),
            ));
        }

        let glyph_texture = Font::get_outlined_glyph_texture(outlined_glyph);
        let add_char_to_font_atlas = |atlas: &mut FontAtlas| -> bool {
            atlas.add_glyph(
                textures,
                texture_atlases,
                glyph_id,
                glyph_position.into(),
                &glyph_texture,
            )
        };
        if !font_atlases.iter_mut().any(add_char_to_font_atlas) {
            // Find the largest dimension of the glyph, either its width or its height
            let glyph_max_size: u32 = glyph_texture
                .texture_descriptor
                .size
                .height
                .max(glyph_texture.texture_descriptor.size.width);
            // Pick the higher  of 512 or the smallest power of 2 greater than glyph_max_size
            let containing = (1u32 << (32 - glyph_max_size.leading_zeros())).max(512) as f32;
            font_atlases.push(FontAtlas::new(
                textures,
                texture_atlases,
                Vec2::new(containing, containing),
            ));
            if !font_atlases.last_mut().unwrap().add_glyph(
                textures,
                texture_atlases,
                glyph_id,
                glyph_position.into(),
                &glyph_texture,
            ) {
                return Err(TextError::FailedToAddGlyph(glyph_id));
            }
        }

        if text_settings.allow_dynamic_font_size {
            // Clear last space in queue to make room for new font size
            while self.queue.len() > text_settings.max_font_atlases.get() {
                if let Some(font_size_key) = self.queue.pop_back() {
                    self.font_atlases.remove(&font_size_key);
                }
            }
        }

        Ok(self
            .get_glyph_atlas_info(font_size, glyph_id, glyph_position)
            .unwrap())
    }

    pub fn get_glyph_atlas_info(
        &mut self,
        font_size: f32,
        glyph_id: GlyphId,
        position: Point,
    ) -> Option<GlyphAtlasInfo> {
        let font_size_key = FloatOrd(font_size);

        self.update_last_used(&font_size_key);

        self.font_atlases
            .get(&font_size_key)
            .and_then(|font_atlases| {
                font_atlases
                    .iter()
                    .find_map(|atlas| {
                        atlas
                            .get_glyph_index(glyph_id, position.into())
                            .map(|glyph_index| (glyph_index, atlas.texture_atlas.clone_weak()))
                    })
                    .map(|(glyph_index, texture_atlas)| GlyphAtlasInfo {
                        texture_atlas,
                        glyph_index,
                    })
            })
    }

    fn update_last_used(&mut self, font_size_key: &FontSizeKey) {
        if let Some(pos) = self.queue.iter().position(|i| *i == *font_size_key) {
            if let Some(key) = self.queue.remove(pos) {
                self.queue.push_front(key);
            }
        } else {
            self.queue.push_front(*font_size_key);
        }
    }
}
