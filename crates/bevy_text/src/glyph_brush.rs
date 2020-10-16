use std::sync::Mutex;

use bevy_asset::{Assets, Handle};
use bevy_math::Vec2;
use glyph_brush_layout::{GlyphPositioner, Layout, SectionGeometry, SectionGlyph, ToSectionText};

use crate::{error::TextError, Font};

pub struct GlyphBrush {
    section_queue: Mutex<Vec<Vec<SectionGlyph>>>,
}

impl GlyphBrush {
    pub fn compute_glyphs<S: ToSectionText>(
        &self,
        font_storage: &Assets<Font>,
        font_handle: &Handle<Font>,
        sections: &[S],
        bounds: Vec2,
    ) -> Result<Vec<SectionGlyph>, TextError> {
        // Todo: handle cache
        let font = font_storage.get(font_handle).ok_or(TextError::NoSuchFont)?;
        let geom = SectionGeometry {
            bounds: (bounds.x(), bounds.y()),
            ..Default::default()
        };
        let section_glyphs = Layout::default().calculate_glyphs(&[&font.font], &geom, sections);
        Ok(section_glyphs)
    }

    pub fn queue_text<S: ToSectionText>(
        &self,
        font_storage: &Assets<Font>,
        font_handle: &Handle<Font>,
        sections: &[S],
        bounds: Vec2,
    ) -> Result<(), TextError> {
        let glyphs = self.compute_glyphs(font_storage, font_handle, sections, bounds)?;
        let mut sq = self
            .section_queue
            .lock()
            .expect("Poisoned Mutex on text queue");
        sq.push(glyphs);
        Ok(())
    }

    pub fn process_queue(&self) -> Result<BrushAction, TextError> {
        let mut sq = self
            .section_queue
            .lock()
            .expect("Poisoned Mutex on text queue");
        let mut sq = std::mem::replace(&mut *sq, Vec::new());
        sq.into_iter().for_each(|glyphs| {
            glyphs.into_iter().for_each(|glyph| {
                let size = glyph.glyph.scale.x;
                let id = glyph.glyph.id;
                // map FontId -> Handle<Font> ??
                //glyph.font_id
            });
        });
        Ok(BrushAction::Redraw)
    }
}

pub struct GlyphVertex {
    position: Vec2,
}

pub enum BrushAction {
    Draw(Vec<GlyphVertex>),
    Redraw,
}
