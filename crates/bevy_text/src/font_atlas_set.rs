use crate::{Font, FontAtlas};
use ab_glyph::ScaleFont;
use bevy_asset::{Assets, Handle};
use bevy_core::float_ord::FloatOrd;
use bevy_render::texture::Texture;
use bevy_sprite::TextureAtlas;
use glam::Vec2;
use std::collections::HashMap;

// work around rust's f32 order/hash limitations
type FontSizeKey = FloatOrd;

#[derive(Default)]
pub struct FontAtlasSet {
    font: Handle<Font>,
    font_atlases: HashMap<FontSizeKey, FontAtlas>,
}

#[derive(Debug)]
pub struct GlyphAtlasInfo {
    pub texture_atlas: Handle<TextureAtlas>,
    pub char_index: u32,
}

impl FontAtlasSet {
    pub fn new(font: Handle<Font>) -> Self {
        Self {
            font,
            font_atlases: HashMap::new(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&FontSizeKey, &FontAtlas)> {
        self.font_atlases.iter()
    }

    pub fn has_char(&self, character: char, font_size: f32) -> bool {
        self.font_atlases
            .get(&FloatOrd(font_size))
            .map_or(false, |font_atlas| {
                font_atlas.get_char_index(character).is_some()
            })
    }

    pub fn add_glyphs_to_atlas(
        &mut self,
        fonts: &Assets<Font>,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Texture>,
        font_size: f32,
        text: &str,
    ) {
        let font = fonts.get(&self.font).unwrap();
        let scaled_font = ab_glyph::Font::as_scaled(&font.font, font_size);
        let font_atlas = self
            .font_atlases
            .entry(FloatOrd(font_size))
            .or_insert_with(|| FontAtlas::new(textures, texture_atlases, Vec2::new(512.0, 512.0)));
        for character in text.chars() {
            if character.is_control() || font_atlas.get_char_index(character).is_some() {
                continue;
            }

            let glyph = scaled_font.scaled_glyph(character);
            if let Some(outlined_glyph) = scaled_font.outline_glyph(glyph) {
                let glyph_texture = Font::get_outlined_glyph_texture(outlined_glyph);
                font_atlas.add_char(textures, texture_atlases, character, &glyph_texture);
            }
        }
    }

    pub fn get_glyph_atlas_info(&self, font_size: f32, character: char) -> Option<GlyphAtlasInfo> {
        self.font_atlases
            .get(&FloatOrd(font_size))
            .and_then(|font_atlas| {
                font_atlas
                    .get_char_index(character)
                    .map(|char_index| GlyphAtlasInfo {
                        texture_atlas: font_atlas.texture_atlas,
                        char_index,
                    })
            })
    }
}
