use crate::{
    compute_text_bounds, error::TextError, glyph_brush::GlyphBrush, scale_value, BreakLineOn, Font,
    FontAtlasSets, JustifyText, PositionedGlyph, Text, TextSection, TextSettings, YAxisOrientation,
};
use ab_glyph::PxScale;
use bevy_asset::{AssetId, Assets, Handle};
use bevy_ecs::component::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_ecs::system::Resource;
use bevy_math::Vec2;
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlasLayout;
use bevy_utils::HashMap;
use glyph_brush_layout::{FontId, GlyphPositioner, SectionGeometry, SectionText, ToSectionText};

#[derive(Default, Resource)]
pub struct TextPipeline {
    brush: GlyphBrush,
    map_font_id: HashMap<AssetId<Font>, FontId>,
}

/// Render information for a corresponding [`Text`] component.
///
///  Contains scaled glyphs and their size. Generated via [`TextPipeline::queue_text`].
#[derive(Component, Clone, Default, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct TextLayoutInfo {
    pub glyphs: Vec<PositionedGlyph>,
    pub logical_size: Vec2,
}

impl TextPipeline {
    pub fn get_or_insert_font_id(&mut self, handle: &Handle<Font>, font: &Font) -> FontId {
        let brush = &mut self.brush;
        *self
            .map_font_id
            .entry(handle.id())
            .or_insert_with(|| brush.add_font(handle.id(), font.font.clone()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn queue_text(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        scale_factor: f32,
        text_alignment: JustifyText,
        linebreak_behavior: BreakLineOn,
        bounds: Vec2,
        font_atlas_sets: &mut FontAtlasSets,
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        textures: &mut Assets<Image>,
        text_settings: &TextSettings,
        y_axis_orientation: YAxisOrientation,
    ) -> Result<TextLayoutInfo, TextError> {
        let mut scaled_fonts = Vec::with_capacity(sections.len());
        let sections = sections
            .iter()
            .map(|section| {
                let font = fonts
                    .get(&section.style.font)
                    .ok_or(TextError::NoSuchFont)?;
                let font_id = self.get_or_insert_font_id(&section.style.font, font);
                let font_size = scale_value(section.style.font_size, scale_factor);

                scaled_fonts.push(ab_glyph::Font::as_scaled(&font.font, font_size));

                let section = SectionText {
                    font_id,
                    scale: PxScale::from(font_size),
                    text: &section.value,
                };

                Ok(section)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let section_glyphs =
            self.brush
                .compute_glyphs(&sections, bounds, text_alignment, linebreak_behavior)?;

        if section_glyphs.is_empty() {
            return Ok(TextLayoutInfo::default());
        }

        let size = compute_text_bounds(&section_glyphs, |index| scaled_fonts[index]).size();

        let h_limit = if bounds.x.is_finite() {
            bounds.x
        } else {
            size.x
        };

        let h_anchor = match text_alignment {
            JustifyText::Left => 0.0,
            JustifyText::Center => h_limit * 0.5,
            JustifyText::Right => h_limit * 1.0,
        }
        .floor();

        let glyphs = self.brush.process_glyphs(
            section_glyphs,
            &sections,
            font_atlas_sets,
            fonts,
            texture_atlases,
            textures,
            text_settings,
            y_axis_orientation,
            h_anchor,
        )?;

        Ok(TextLayoutInfo {
            glyphs,
            logical_size: size,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TextMeasureSection {
    pub text: Box<str>,
    pub scale: f32,
    pub font_id: FontId,
}

#[derive(Debug, Clone, Default)]
pub struct TextMeasureInfo {
    pub fonts: Box<[ab_glyph::FontArc]>,
    pub sections: Box<[TextMeasureSection]>,
    pub justification: JustifyText,
    pub linebreak_behavior: glyph_brush_layout::BuiltInLineBreaker,
    pub min: Vec2,
    pub max: Vec2,
}

impl TextMeasureInfo {
    pub fn from_text(
        text: &Text,
        fonts: &Assets<Font>,
        scale_factor: f32,
    ) -> Result<TextMeasureInfo, TextError> {
        let sections = &text.sections;
        let mut auto_fonts = Vec::with_capacity(sections.len());
        let mut out_sections = Vec::with_capacity(sections.len());
        for (i, section) in sections.iter().enumerate() {
            match fonts.get(&section.style.font) {
                Some(font) => {
                    auto_fonts.push(font.font.clone());
                    out_sections.push(TextMeasureSection {
                        font_id: FontId(i),
                        scale: scale_value(section.style.font_size, scale_factor),
                        text: section.value.clone().into_boxed_str(),
                    });
                }
                None => return Err(TextError::NoSuchFont),
            }
        }

        Ok(Self::new(
            auto_fonts,
            out_sections,
            text.justify,
            text.linebreak_behavior.into(),
        ))
    }
    fn new(
        fonts: Vec<ab_glyph::FontArc>,
        sections: Vec<TextMeasureSection>,
        justification: JustifyText,
        linebreak_behavior: glyph_brush_layout::BuiltInLineBreaker,
    ) -> Self {
        let mut info = Self {
            fonts: fonts.into_boxed_slice(),
            sections: sections.into_boxed_slice(),
            justification,
            linebreak_behavior,
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        };

        let min = info.compute_size(Vec2::new(0.0, f32::INFINITY));
        let max = info.compute_size(Vec2::INFINITY);
        info.min = min;
        info.max = max;
        info
    }

    pub fn compute_size(&self, bounds: Vec2) -> Vec2 {
        let sections = &self.sections;
        let geom = SectionGeometry {
            bounds: (bounds.x, bounds.y),
            ..Default::default()
        };
        let section_glyphs = glyph_brush_layout::Layout::default()
            .h_align(self.justification.into())
            .line_breaker(self.linebreak_behavior)
            .calculate_glyphs(&self.fonts, &geom, sections);

        compute_text_bounds(&section_glyphs, |index| {
            let font = &self.fonts[index];
            let font_size = self.sections[index].scale;
            ab_glyph::Font::into_scaled(font, font_size)
        })
        .size()
    }
}
impl ToSectionText for TextMeasureSection {
    #[inline(always)]
    fn to_section_text(&self) -> SectionText<'_> {
        SectionText {
            text: &self.text,
            scale: PxScale::from(self.scale),
            font_id: self.font_id,
        }
    }
}
