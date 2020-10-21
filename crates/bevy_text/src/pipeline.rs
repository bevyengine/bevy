use std::collections::HashMap;

use ab_glyph::PxScale;
use bevy_asset::{Assets, Handle};
use bevy_math::{Size, Vec2};
use bevy_render::prelude::Texture;
use bevy_sprite::TextureAtlas;

use glyph_brush_layout::FontId;

use crate::{
    error::TextError, glyph_brush::BrushAction, glyph_brush::GlyphBrush, Font, FontAtlasSet,
};

pub struct TextPipeline {
    pub brush: GlyphBrush,
    pub map_font_id: HashMap<Handle<Font>, FontId>,
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
        font_handle: &Handle<Font>,
        font_storage: &Assets<Font>,
        text: &str,
        size: f32,
        bounds: Size,
    ) -> Result<Vec2, TextError> {
        let font = font_storage.get(font_handle).ok_or(TextError::NoSuchFont)?;
        let font_id = self.get_or_insert_font_id(font_handle, font);

        let section = glyph_brush_layout::SectionText {
            font_id,
            scale: PxScale::from(size),
            text,
        };

        let glyphs = self.brush.compute_glyphs(&[section], bounds, Vec2::new(0.,0.))?;
        /*
        self.measure_brush
            .borrow_mut()
            .glyph_bounds(section)
            .map(|bounds| (bounds.width().ceil(), bounds.height().ceil()))
            */

        // todo: calculate total bounds

        todo!()
    }

    pub fn get_or_insert_font_id(&mut self, handle: &Handle<Font>, font: &Font) -> FontId {
        self.map_font_id
            .entry(*handle)
            .or_insert(self.brush.add_font(handle.clone(), font.font.clone()))
            .clone()
    }

    pub fn queue_text(
        &mut self,
        font_handle: &Handle<Font>,
        font_storage: &Assets<Font>,
        text: &str,
        size: f32,
        bounds: Size,
        screen_position: Vec2,
    ) -> Result<(), TextError> {
        let font = font_storage.get(font_handle).ok_or(TextError::NoSuchFont)?;
        let font_id = self.get_or_insert_font_id(font_handle, font);

        println!("the font id is {:?}", font_id);

        let section = glyph_brush_layout::SectionText {
            font_id,
            scale: PxScale::from(size),
            text,
        };

        self.brush.queue_text(&[section], bounds, screen_position)?;

        println!("queued text");

        Ok(())
    }

    pub fn draw_queued(
        &self,
        fonts: &Assets<Font>,
        font_atlas_set_storage: &mut Assets<FontAtlasSet>,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Texture>,
    ) -> Result<BrushAction, TextError> {
        self.brush
            .process_queued(font_atlas_set_storage, fonts, texture_atlases, textures)
    }
}
