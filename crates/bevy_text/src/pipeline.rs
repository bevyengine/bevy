use std::{cell::RefCell, collections::HashMap};

use ab_glyph::FontArc;
use bevy_asset::{Assets, Handle};
use bevy_ecs::ResMut;
use bevy_math::Vec2;
use bevy_render::{prelude::Texture, texture::TextureFormat};
use bevy_sprite::TextureAtlas;
use glyph_brush::{BrushAction, BrushError, FontId, GlyphBrush, GlyphBrushBuilder};

use crate::{Font, FontAtlasSet};

pub struct TextPipeline {
    pub draw_brush: RefCell<GlyphBrush<FontArc>>,
    pub measure_brush: RefCell<GlyphBrush<FontArc>>,
    pub map_font_id: HashMap<Handle<Font>, FontId>,
}

impl Default for TextPipeline {
    fn default() -> Self {
        let draw_brush = GlyphBrushBuilder::using_fonts::<FontArc>(vec![]).build();
        let draw_brush = RefCell::new(draw_brush);
        let measure_brush = GlyphBrushBuilder::using_fonts::<FontArc>(vec![]).build();
        let measure_brush = RefCell::new(measure_brush);
        let map_font_id = HashMap::default();
        TextPipeline {
            measure_brush,
            draw_brush,
            map_font_id,
        }
    }
}

impl TextPipeline {
    pub fn measure(
        &self,
        font_handle: &Handle<Font>,
        font_storage: &Assets<Font>,
        contents: &str,
        size: f32,
        bounds: Vec2,
    ) -> Option<(f32, f32)> {
        use glyph_brush::GlyphCruncher;
        let font = font_storage.get(font_handle)?;
        let font_id = self.get_or_insert_font_id(font_handle, font);

        let section = glyph_brush::Section {
            bounds: (bounds.x(), bounds.y()),
            text: vec![glyph_brush::Text {
                text: contents,
                scale: size.into(),
                font_id,
                extra: glyph_brush::Extra::default(),
            }],
            // todo: handle Layout (h_align, v_align)
            ..Default::default()
        };

        self.measure_brush
            .borrow_mut()
            .glyph_bounds(section)
            .map(|bounds| (bounds.width().ceil(), bounds.height().ceil()))
    }

    pub fn get_or_insert_font_id(&self, handle: &Handle<Font>, font: &Font) -> FontId {
        if let Some(font_id) = self.map_font_id.get(handle) {
            return font_id.clone();
        }

        let _ = self.draw_brush.borrow_mut().add_font(font.font.clone());
        self.measure_brush.borrow_mut().add_font(font.font.clone())
    }

    pub fn queue_text(
        &self,
        font_handle: &Handle<Font>,
        font_storage: &Assets<Font>,
        contents: &str,
        size: f32,
        bounds: Vec2,
    ) {
        /*
        let mut draw_brush = self.draw_brush.borrow_mut();
        let font = font_storage.get(font_handle);
        if font.is_none() {
            return;
        }
        let font_id = self.get_or_insert_font_id(font_handle, font.unwrap());

        let section = glyph_brush::Section {
            bounds: (bounds.x(), bounds.y()),
            text: vec![glyph_brush::Text {
                text: contents,
                scale: size.into(),
                font_id,
                extra: glyph_brush::Extra::default(),
            }],
            // todo: handle Layout (h_align, v_align)
            ..Default::default()
        };

        draw_brush.queue(section);
        */
    }

    pub fn draw_queued(
        &self,
        textures: &mut Assets<Texture>,
        texture_atlases: &mut Assets<TextureAtlas>,
        font_atlas_set: &mut FontAtlasSet,
    ) -> Result<BrushAction<FontArc>, BrushError> {
        let mut draw_brush = self.draw_brush.borrow_mut();

        draw_brush.process_queued(
            |bounds, data| {
                let texture = Texture::new(
                    Vec2::new(bounds.width() as f32, bounds.height() as f32),
                    data.to_owned(),
                    TextureFormat::Rgba8UnormSrgb,
                );
                font_atlas_set.add_texture_to_atlas(textures, texture_atlases, texture)
            },
            |_vertex| todo!(),
        )
    }
}

pub struct TextVertex {
    translation: Vec2,
    color: u32,
    atlas_index: usize,
    tex_index_on_atlas: u32,
}
