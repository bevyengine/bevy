use std::sync::Mutex;

use ab_glyph::FontArc;
use bevy_asset::{Assets, Handle};
use bevy_math::{Size, Vec2};
use bevy_render::prelude::Texture;
use bevy_sprite::TextureAtlas;
use glyph_brush_layout::{
    FontId, GlyphPositioner, Layout, SectionGeometry, SectionGlyph, ToSectionText,
};

use crate::{error::TextError, Font, FontAtlasSet, GlyphAtlasInfo};

pub struct GlyphBrush {
    section_queue: Mutex<Vec<Vec<SectionGlyph>>>,
    fonts: Vec<FontArc>,
    handles: Vec<Handle<Font>>,
    latest_font_id: FontId,
}

impl Default for GlyphBrush {
    fn default() -> Self {
        GlyphBrush {
            section_queue: Mutex::new(Vec::new()),
            fonts: Vec::new(),
            handles: Vec::new(),
            latest_font_id: FontId(0),
        }
    }
}

impl GlyphBrush {
    pub fn compute_glyphs<S: ToSectionText>(
        &self,
        sections: &[S],
        bounds: Size,
        screen_position: Vec2,
    ) -> Result<Vec<SectionGlyph>, TextError> {
        // Todo: handle cache
        let geom = SectionGeometry {
            bounds: (bounds.width, bounds.height),
            screen_position: (screen_position.x(), screen_position.y()),
            ..Default::default()
        };
        let section_glyphs = Layout::default().calculate_glyphs(&self.fonts, &geom, sections);
        Ok(section_glyphs)
    }

    pub fn queue_text<S: ToSectionText>(
        &self,
        sections: &[S],
        bounds: Size,
        screen_position: Vec2,
    ) -> Result<(), TextError> {
        let glyphs = self.compute_glyphs(sections, bounds, screen_position)?;
        let mut sq = self
            .section_queue
            .lock()
            .expect("Poisoned Mutex on text queue");
        sq.push(glyphs);
        Ok(())
    }

    pub fn process_queued(
        &self,
        font_atlas_set_storage: &mut Assets<FontAtlasSet>,
        fonts: &Assets<Font>,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Texture>,
    ) -> Result<Vec<TextVertex>, TextError> {
        let mut sq = self
            .section_queue
            .lock()
            .expect("Poisoned Mutex on text queue");
        let sq = std::mem::replace(&mut *sq, Vec::new());
        let vertices = sq
            .into_iter()
            .map(|glyphs| {
                let mut vertices = Vec::new();
                for glyph in glyphs {
                    let handle = &self.handles[glyph.font_id.0];
                    let handle_font_atlas: Handle<FontAtlasSet> = handle.as_weak();
                    let font_atlas_set = font_atlas_set_storage
                        .get_or_insert_with(handle_font_atlas, || {
                            FontAtlasSet::new(handle.clone())
                        });
                    let position = glyph.glyph.position;
                    let position = Vec2::new(position.x + glyph.glyph.scale.x/2., position.y - glyph.glyph.scale.y - glyph.glyph.scale.y /2. );
                    let atlas_info = font_atlas_set
                        .get_glyph_atlas_info(glyph.glyph.scale.y, glyph.glyph.id)
                        .map(|gaf| Ok(gaf))
                        .unwrap_or_else(|| {
                            font_atlas_set.add_glyph_to_atlas(
                                fonts,
                                texture_atlases,
                                textures,
                                glyph.glyph,
                            )
                        })?;
                    vertices.push(TextVertex {
                        position,
                        atlas_info,
                    });
                }
                Ok(vertices)
            })
            .collect::<Result<Vec<_>, TextError>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(vertices)
    }

    pub fn add_font(&mut self, handle: Handle<Font>, font: FontArc) -> FontId {
        self.fonts.push(font);
        self.handles.push(handle);
        let font_id = self.latest_font_id;
        self.latest_font_id = FontId(font_id.0 + 1);
        font_id
    }
}

#[derive(Debug, Clone)]
pub struct TextVertex {
    pub position: Vec2,
    pub atlas_info: GlyphAtlasInfo,
}

#[derive(Debug, Default, Clone)]
pub struct TextVertices(Vec<TextVertex>);

impl TextVertices {
    pub fn borrow(&self) -> &Vec<TextVertex> {
        &self.0
    }

    pub fn set(&mut self, vertices: Vec<TextVertex>) {
        self.0 = vertices;
    }
}
