use crate::{error::TextError, Font, FontAtlas};
use ab_glyph::{GlyphId, OutlinedGlyph};
use bevy_asset::{Assets, Handle};
use bevy_core::FloatOrd;
use bevy_math::Vec2;
use bevy_render::texture::Texture;
use bevy_sprite::TextureAtlas;
use bevy_type_registry::TypeUuid;
use bevy_utils::{HashMap, HashMapExt};

type FontSizeKey = FloatOrd;

#[derive(TypeUuid)]
#[uuid = "73ba778b-b6b5-4f45-982d-d21b6b86ace2"]
pub struct FontAtlasSet {
    font_atlases: HashMap<FontSizeKey, Vec<FontAtlas>>,
}

#[derive(Debug, Clone)]
pub struct GlyphAtlasInfo {
    pub texture_atlas: Handle<TextureAtlas>,
    pub glyph_index: u32,
}

impl Default for FontAtlasSet {
    fn default() -> Self {
        FontAtlasSet {
            font_atlases: HashMap::with_capacity(1),
        }
    }
}

impl FontAtlasSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&FontSizeKey, &Vec<FontAtlas>)> {
        self.font_atlases.iter()
    }

    pub fn has_glyph(&self, glyph_id: GlyphId, font_size: f32) -> bool {
        self.font_atlases
            .get(&FloatOrd(font_size))
            .map_or(false, |font_atlas| {
                font_atlas.iter().any(|atlas| atlas.has_glyph(glyph_id))
            })
    }

    pub fn add_glyph_to_atlas(
        &mut self,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Texture>,
        outlined_glyph: OutlinedGlyph,
    ) -> Result<GlyphAtlasInfo, TextError> {
        let glyph = outlined_glyph.glyph();
        let glyph_id = glyph.id;
        let font_size = glyph.scale.y;
        let font_atlases = self
            .font_atlases
            .entry(FloatOrd(font_size))
            .or_insert_with(|| {
                vec![FontAtlas::new(
                    textures,
                    texture_atlases,
                    Vec2::new(512.0, 512.0),
                )]
            });
        let glyph_texture = Font::get_outlined_glyph_texture(outlined_glyph);
        let add_char_to_font_atlas = |atlas: &mut FontAtlas| -> bool {
            atlas.add_glyph(textures, texture_atlases, glyph_id, &glyph_texture)
        };
        if !font_atlases.iter_mut().any(add_char_to_font_atlas) {
            font_atlases.push(FontAtlas::new(
                textures,
                texture_atlases,
                Vec2::new(512.0, 512.0),
            ));
            if !font_atlases.last_mut().unwrap().add_glyph(
                textures,
                texture_atlases,
                glyph_id,
                &glyph_texture,
            ) {
                panic!("could not add character to newly created FontAtlas");
            }
        }

        Ok(self.get_glyph_atlas_info(font_size, glyph_id).unwrap())
    }

    pub fn get_glyph_atlas_info(
        &self,
        font_size: f32,
        glyph_id: GlyphId,
    ) -> Option<GlyphAtlasInfo> {
        self.font_atlases
            .get(&FloatOrd(font_size))
            .and_then(|font_atlases| {
                font_atlases
                    .iter()
                    .find_map(|atlas| {
                        atlas
                            .get_glyph_index(glyph_id)
                            .map(|glyph_index| (glyph_index, atlas.texture_atlas.clone_weak()))
                    })
                    .map(|(glyph_index, texture_atlas)| GlyphAtlasInfo {
                        texture_atlas,
                        glyph_index,
                    })
            })
    }
}
