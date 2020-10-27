use std::collections::HashMap;

use ab_glyph::{PxScale, ScaleFont};
use bevy_asset::{Assets, Handle, HandleId};
use bevy_math::Size;
use bevy_render::prelude::Texture;
use bevy_sprite::TextureAtlas;

use glyph_brush_layout::{FontId, SectionText};

use crate::{error::TextError, glyph_brush::GlyphBrush, Font, FontAtlasSet, TextVertex};

pub struct TextPipeline {
    pub brush: GlyphBrush,
    pub map_font_id: HashMap<HandleId, FontId>,
}

impl Default for TextPipeline {
    fn default() -> Self {
        let brush = GlyphBrush::default();
        let map_font_id = HashMap::default();
        TextPipeline { brush, map_font_id }
    }
}

impl TextPipeline {
    pub fn measure(
        &mut self,
        font_handle: Handle<Font>,
        font_storage: &Assets<Font>,
        text: &str,
        scale: f32,
        bounds: Size,
    ) -> Result<Size<f32>, TextError> {
        let font = font_storage
            .get(font_handle.id)
            .ok_or(TextError::NoSuchFont)?;

        let font_id = self.get_or_insert_font_id(font_handle, font);

        let section = SectionText {
            font_id,
            scale: PxScale::from(scale),
            text,
        };

        let scaled_font = ab_glyph::Font::as_scaled(&font.font, scale);

        let section_glyphs = self.brush.compute_glyphs(&[section], bounds)?;

        let mut max_x: f32 = 0.0;
        let mut max_y: f32 = 0.0;
        for section_glyph in section_glyphs.iter() {
            let glyph = &section_glyph.glyph;
            max_x = max_x.max(glyph.position.x + scaled_font.h_advance(glyph.id));
            max_y = max_y.max(glyph.position.y - scaled_font.descent());
        }
        let size = Size::new(max_x, max_y);
        Ok(size)
    }

    pub fn get_or_insert_font_id(&mut self, handle: Handle<Font>, font: &Font) -> FontId {
        let brush = &mut self.brush;
        self.map_font_id
            .entry(handle.id)
            .or_insert_with(|| brush.add_font(handle.clone(), font.font.clone()))
            .clone()
    }

    pub fn queue_text(
        &mut self,
        font_handle: Handle<Font>,
        font_storage: &Assets<Font>,
        text: &str,
        font_size: f32,
        bounds: Size,
        // screen_position: Vec2,
    ) -> Result<(), TextError> {
        let font = font_storage
            .get(font_handle.id)
            .ok_or(TextError::NoSuchFont)?;
        let font_id = self.get_or_insert_font_id(font_handle, font);

        let section = SectionText {
            font_id,
            scale: PxScale::from(font_size),
            text,
        };

        self.brush.queue_text(&[section], bounds)?;

        Ok(())
    }

    pub fn process_queued(
        &self,
        fonts: &Assets<Font>,
        font_atlas_set_storage: &mut Assets<FontAtlasSet>,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Texture>,
    ) -> Result<Vec<TextVertex>, TextError> {
        self.brush
            .process_queued(font_atlas_set_storage, fonts, texture_atlases, textures)
    }
}
