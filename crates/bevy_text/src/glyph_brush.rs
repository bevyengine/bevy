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
    fonts: Vec<FontArc>,
    handles: Vec<Handle<Font>>,
    latest_font_id: FontId,
}

impl Default for GlyphBrush {
    fn default() -> Self {
        GlyphBrush {
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

    pub fn process_glyphs(
        &self,
        glyphs: Vec<SectionGlyph>,
        font_atlas_set_storage: &mut Assets<FontAtlasSet>,
        fonts: &Assets<Font>,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Texture>,
    ) -> Result<Vec<PositionedGlyph>, TextError> {
        if glyphs.is_empty() {
            return Ok(Vec::new());
        }

        let first_glyph = glyphs.first().expect("Must have at least one glyph.");
        let font_id = first_glyph.font_id.0;
        let handle = &self.handles[font_id];
        let font = fonts.get(handle).ok_or(TextError::NoSuchFont)?;
        let font_size = first_glyph.glyph.scale.y;
        let scaled_font = ab_glyph::Font::as_scaled(&font.font, font_size);
        let mut max_y = std::f32::MIN;
        let mut min_x = std::f32::MAX;
        for section_glyph in glyphs.iter() {
            let glyph = &section_glyph.glyph;
            max_y = max_y.max(glyph.position.y - scaled_font.descent());
            min_x = min_x.min(glyph.position.x);
        }
        max_y = max_y.floor();
        min_x = min_x.floor();

        let mut positioned_glyphs = Vec::new();
        for sg in glyphs {
            let glyph_id = sg.glyph.id;
            if let Some(outlined_glyph) = font.font.outline_glyph(sg.glyph) {
                let bounds = outlined_glyph.px_bounds();
                let handle_font_atlas: Handle<FontAtlasSet> = handle.as_weak();
                let font_atlas_set = font_atlas_set_storage
                    .get_or_insert_with(handle_font_atlas, FontAtlasSet::default);

                let atlas_info = font_atlas_set
                    .get_glyph_atlas_info(font_size, glyph_id)
                    .map(Ok)
                    .unwrap_or_else(|| {
                        font_atlas_set.add_glyph_to_atlas(texture_atlases, textures, outlined_glyph)
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

                positioned_glyphs.push(PositionedGlyph {
                    position,
                    atlas_info,
                });
            }
        }
        Ok(positioned_glyphs)
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
pub struct PositionedGlyph {
    pub position: Vec2,
    pub atlas_info: GlyphAtlasInfo,
}
