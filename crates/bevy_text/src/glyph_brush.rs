use std::sync::Mutex;

use ab_glyph::{Font as _, FontArc, ScaleFont as _};
use bevy_asset::{Assets, Handle};
use bevy_math::{Size, Vec2};
use bevy_render::prelude::Texture;
use bevy_sprite::TextureAtlas;
use glyph_brush_layout::{
    FontId, GlyphPositioner, Layout, SectionGeometry, SectionGlyph, ToSectionText,
};

use crate::{error::TextError, Font, FontAtlasSet, GlyphAtlasInfo, TextAlignment};

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
        text_alignment: TextAlignment,
    ) -> Result<Vec<SectionGlyph>, TextError> {
        // Todo: handle cache
        let geom = SectionGeometry {
            bounds: (bounds.width, bounds.height),
            ..Default::default()
        };
        let section_glyphs = Layout::default()
            .h_align(text_alignment.horizontal)
            .v_align(text_alignment.vertical)
            .calculate_glyphs(&self.fonts, &geom, sections);
        Ok(section_glyphs)
    }

    pub fn queue_text<S: ToSectionText>(
        &self,
        sections: &[S],
        bounds: Size,
        text_alignment: TextAlignment,
    ) -> Result<(), TextError> {
        let glyphs = self.compute_glyphs(sections, bounds, text_alignment)?;
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
            .map(|section_glyphs| {
                if section_glyphs.is_empty() {
                    return Ok(Vec::new());
                }
                let mut vertices = Vec::new();
                let sg = section_glyphs.first().unwrap();
                let mut min_x: f32 = sg.glyph.position.x;
                let mut max_y: f32 = sg.glyph.position.y;
                for section_glyph in section_glyphs.iter() {
                    let handle = &self.handles[section_glyph.font_id.0];
                    let font = fonts.get(handle).ok_or(TextError::NoSuchFont)?;
                    let glyph = &section_glyph.glyph;
                    let scaled_font = ab_glyph::Font::as_scaled(&font.font, glyph.scale.y);
                    // glyph.position.y = baseline
                    max_y = max_y.max(glyph.position.y - scaled_font.descent());
                    min_x = min_x.min(glyph.position.x - scaled_font.h_side_bearing(glyph.id));
                }
                max_y = max_y.floor();
                min_x = min_x.floor();

                for section_glyph in section_glyphs {
                    let handle = &self.handles[section_glyph.font_id.0];
                    let font = fonts.get(handle).ok_or(TextError::NoSuchFont)?;
                    let glyph_id = section_glyph.glyph.id;
                    let font_size = section_glyph.glyph.scale.y;
                    if let Some(outlined_glyph) = font.font.outline_glyph(section_glyph.glyph) {
                        let bounds = outlined_glyph.px_bounds();
                        let handle_font_atlas: Handle<FontAtlasSet> = handle.as_weak();
                        let font_atlas_set = font_atlas_set_storage
                            .get_or_insert_with(handle_font_atlas, FontAtlasSet::default);

                        let atlas_info = font_atlas_set
                            .get_glyph_atlas_info(font_size, glyph_id)
                            .map(Ok)
                            .unwrap_or_else(|| {
                                font_atlas_set.add_glyph_to_atlas(
                                    texture_atlases,
                                    textures,
                                    outlined_glyph,
                                )
                            })?;

                        let texture_atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();
                        let glyph_rect = texture_atlas.textures[atlas_info.glyph_index as usize];
                        let glyph_width = glyph_rect.width();
                        let glyph_height = glyph_rect.height();

                        let x = bounds.min.x + glyph_width / 2.0 - min_x;
                        // the 0.5 accounts for odd-numbered heights (bump up by 1 pixel)
                        // max_y = text block height, and up is negative (whereas for transform, up is positive)
                        let y = max_y - bounds.max.y + glyph_height / 2.0 + 0.5;
                        let position = Vec2::new(x, y);

                        vertices.push(TextVertex {
                            position,
                            atlas_info,
                        });
                    }
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

impl std::ops::Deref for TextVertices {
    type Target = Vec<TextVertex>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TextVertices {
    pub fn set(&mut self, vertices: Vec<TextVertex>) {
        self.0 = vertices;
    }
}
