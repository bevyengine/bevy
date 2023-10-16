use ab_glyph::{Font as _, FontArc, Glyph, PxScaleFont, ScaleFont as _};
use bevy_asset::{AssetId, Assets};
use bevy_math::{Rect, Vec2};
use bevy_reflect::Reflect;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_utils::tracing::warn;
use glyph_brush_layout::{
    BuiltInLineBreaker, FontId, GlyphPositioner, Layout, SectionGeometry, SectionGlyph,
    SectionText, ToSectionText,
};

use crate::{
    error::TextError, BreakLineOn, Font, FontAtlasSet, FontAtlasSets, FontAtlasWarning,
    GlyphAtlasInfo, TextAlignment, TextSettings, YAxisOrientation,
};

pub struct GlyphBrush {
    fonts: Vec<FontArc>,
    asset_ids: Vec<AssetId<Font>>,
    latest_font_id: FontId,
}

impl Default for GlyphBrush {
    fn default() -> Self {
        GlyphBrush {
            fonts: Vec::new(),
            asset_ids: Vec::new(),
            latest_font_id: FontId(0),
        }
    }
}

impl GlyphBrush {
    pub fn compute_glyphs<S: ToSectionText>(
        &self,
        sections: &[S],
        bounds: Vec2,
        text_alignment: TextAlignment,
        linebreak_behavior: BreakLineOn,
    ) -> Result<Vec<SectionGlyph>, TextError> {
        let geom = SectionGeometry {
            bounds: (bounds.x, bounds.y),
            ..Default::default()
        };

        let lbb: BuiltInLineBreaker = linebreak_behavior.into();

        let section_glyphs = Layout::default()
            .h_align(text_alignment.into())
            .line_breaker(lbb)
            .calculate_glyphs(&self.fonts, &geom, sections);
        Ok(section_glyphs)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process_glyphs(
        &self,
        glyphs: Vec<SectionGlyph>,
        sections: &[SectionText],
        font_atlas_sets: &mut FontAtlasSets,
        fonts: &Assets<Font>,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Image>,
        text_settings: &TextSettings,
        font_atlas_warning: &mut FontAtlasWarning,
        y_axis_orientation: YAxisOrientation,
    ) -> Result<Vec<PositionedGlyph>, TextError> {
        if glyphs.is_empty() {
            return Ok(Vec::new());
        }

        let sections_data = sections
            .iter()
            .map(|section| {
                let asset_id = &self.asset_ids[section.font_id.0];
                let font = fonts.get(*asset_id).ok_or(TextError::NoSuchFont)?;
                let font_size = section.scale.y;
                Ok((
                    asset_id,
                    font,
                    font_size,
                    ab_glyph::Font::as_scaled(&font.font, font_size),
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let text_bounds = compute_text_bounds(&glyphs, |index| sections_data[index].3);

        let mut positioned_glyphs = Vec::new();
        for sg in glyphs {
            let SectionGlyph {
                section_index: _,
                byte_index,
                mut glyph,
                font_id: _,
            } = sg;
            let glyph_id = glyph.id;
            let glyph_position = glyph.position;
            let adjust = GlyphPlacementAdjuster::new(&mut glyph);
            let section_data = sections_data[sg.section_index];
            if let Some(outlined_glyph) = section_data.1.font.outline_glyph(glyph) {
                let bounds = outlined_glyph.px_bounds();
                let font_atlas_set = font_atlas_sets
                    .sets
                    .entry(*section_data.0)
                    .or_insert_with(FontAtlasSet::default);

                let atlas_info = font_atlas_set
                    .get_glyph_atlas_info(section_data.2, glyph_id, glyph_position)
                    .map(Ok)
                    .unwrap_or_else(|| {
                        font_atlas_set.add_glyph_to_atlas(texture_atlases, textures, outlined_glyph)
                    })?;

                if !text_settings.allow_dynamic_font_size
                    && !font_atlas_warning.warned
                    && font_atlas_set.len() > text_settings.max_font_atlases.get()
                {
                    warn!("warning[B0005]: Number of font atlases has exceeded the maximum of {}. Performance and memory usage may suffer.", text_settings.max_font_atlases.get());
                    font_atlas_warning.warned = true;
                }

                let texture_atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();
                let glyph_rect = texture_atlas.textures[atlas_info.glyph_index];
                let size = Vec2::new(glyph_rect.width(), glyph_rect.height());

                let x = bounds.min.x + size.x / 2.0 - text_bounds.min.x;

                let y = match y_axis_orientation {
                    YAxisOrientation::BottomToTop => {
                        text_bounds.max.y - bounds.max.y + size.y / 2.0
                    }
                    YAxisOrientation::TopToBottom => {
                        bounds.min.y + size.y / 2.0 - text_bounds.min.y
                    }
                };

                let position = adjust.position(Vec2::new(x, y));

                positioned_glyphs.push(PositionedGlyph {
                    position,
                    size,
                    atlas_info,
                    section_index: sg.section_index,
                    byte_index,
                });
            }
        }
        Ok(positioned_glyphs)
    }

    pub fn add_font(&mut self, asset_id: AssetId<Font>, font: FontArc) -> FontId {
        self.fonts.push(font);
        self.asset_ids.push(asset_id);
        let font_id = self.latest_font_id;
        self.latest_font_id = FontId(font_id.0 + 1);
        font_id
    }
}

#[derive(Debug, Clone, Reflect)]
pub struct PositionedGlyph {
    pub position: Vec2,
    pub size: Vec2,
    pub atlas_info: GlyphAtlasInfo,
    pub section_index: usize,
    pub byte_index: usize,
}

#[cfg(feature = "subpixel_glyph_atlas")]
struct GlyphPlacementAdjuster;

#[cfg(feature = "subpixel_glyph_atlas")]
impl GlyphPlacementAdjuster {
    #[inline(always)]
    pub fn new(_: &mut Glyph) -> Self {
        Self
    }

    #[inline(always)]
    pub fn position(&self, p: Vec2) -> Vec2 {
        p
    }
}

#[cfg(not(feature = "subpixel_glyph_atlas"))]
struct GlyphPlacementAdjuster(f32);

#[cfg(not(feature = "subpixel_glyph_atlas"))]
impl GlyphPlacementAdjuster {
    #[inline(always)]
    pub fn new(glyph: &mut Glyph) -> Self {
        let v = glyph.position.x.round();
        glyph.position.x = 0.;
        glyph.position.y = glyph.position.y.ceil();
        Self(v)
    }

    #[inline(always)]
    pub fn position(&self, v: Vec2) -> Vec2 {
        Vec2::new(self.0, 0.) + v
    }
}

/// Computes the minimal bounding rectangle for a block of text.
/// Ignores empty trailing lines.
pub(crate) fn compute_text_bounds<T>(
    section_glyphs: &[SectionGlyph],
    get_scaled_font: impl Fn(usize) -> PxScaleFont<T>,
) -> bevy_math::Rect
where
    T: ab_glyph::Font,
{
    let mut text_bounds = Rect {
        min: Vec2::splat(std::f32::MAX),
        max: Vec2::splat(std::f32::MIN),
    };

    for sg in section_glyphs {
        let scaled_font = get_scaled_font(sg.section_index);
        let glyph = &sg.glyph;
        text_bounds = text_bounds.union(Rect {
            min: Vec2::new(glyph.position.x, 0.),
            max: Vec2::new(
                glyph.position.x + scaled_font.h_advance(glyph.id),
                glyph.position.y - scaled_font.descent(),
            ),
        });
    }

    text_bounds
}
