use ab_glyph::PxScale;
use bevy_asset::{Assets, Handle, HandleId};
use bevy_ecs::component::Component;
use bevy_ecs::system::Resource;
use bevy_math::Vec2;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_utils::HashMap;

use glyph_brush_layout::{FontId, GlyphPositioner, SectionGeometry, SectionText};

use crate::{
    compute_text_bounds, error::TextError, glyph_brush::GlyphBrush, scale_value, BreakLineOn, Font,
    FontAtlasSet, FontAtlasWarning, PositionedGlyph, TextAlignment, TextSection, TextSettings,
    YAxisOrientation,
};

#[derive(Default, Resource)]
pub struct TextPipeline {
    brush: GlyphBrush,
    map_font_id: HashMap<HandleId, FontId>,
}

/// Render information for a corresponding [`Text`](crate::Text) component.
///
///  Contains scaled glyphs and their size. Generated via [`TextPipeline::queue_text`].
#[derive(Component, Clone, Default, Debug)]
pub struct TextLayoutInfo {
    pub glyphs: Vec<PositionedGlyph>,
    pub size: Vec2,
}

impl TextPipeline {
    pub fn get_or_insert_font_id(&mut self, handle: &Handle<Font>, font: &Font) -> FontId {
        let brush = &mut self.brush;
        *self
            .map_font_id
            .entry(handle.id())
            .or_insert_with(|| brush.add_font(handle.clone(), font.font.clone()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn queue_text(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        scale_factor: f64,
        text_alignment: TextAlignment,
        linebreak_behavior: BreakLineOn,
        bounds: Vec2,
        font_atlas_set_storage: &mut Assets<FontAtlasSet>,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Image>,
        text_settings: &TextSettings,
        font_atlas_warning: &mut FontAtlasWarning,
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

        let size = compute_text_bounds(&section_glyphs, |index| &scaled_fonts[index]).size();

        let glyphs = self.brush.process_glyphs(
            section_glyphs,
            &sections,
            font_atlas_set_storage,
            fonts,
            texture_atlases,
            textures,
            text_settings,
            font_atlas_warning,
            y_axis_orientation,
        )?;

        Ok(TextLayoutInfo { glyphs, size })
    }

    pub fn create_text_measure(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        scale_factor: f64,
        text_alignment: TextAlignment,
        linebreak_behaviour: BreakLineOn,
    ) -> Result<TextMeasureInfo, TextError> {
        let mut auto_fonts = Vec::with_capacity(sections.len());
        let mut scaled_fonts = Vec::with_capacity(sections.len());
        let sections = sections
            .iter()
            .enumerate()
            .map(|(i, section)| {
                let font = fonts
                    .get(&section.style.font)
                    .ok_or(TextError::NoSuchFont)?;
                let font_size = scale_value(section.style.font_size, scale_factor);
                auto_fonts.push(font.font.clone());
                let px_scale_font = ab_glyph::Font::into_scaled(font.font.clone(), font_size);
                scaled_fonts.push(px_scale_font);

                let section = TextMeasureSection {
                    font_id: FontId(i),
                    scale: PxScale::from(font_size),
                    text: section.value.clone(),
                };

                Ok(section)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(TextMeasureInfo::new(
            auto_fonts,
            scaled_fonts,
            sections,
            text_alignment,
            linebreak_behaviour.into(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct TextMeasureSection {
    pub text: String,
    pub scale: PxScale,
    pub font_id: FontId,
}

#[derive(Debug, Clone)]
pub struct TextMeasureInfo {
    pub fonts: Vec<ab_glyph::FontArc>,
    pub scaled_fonts: Vec<ab_glyph::PxScaleFont<ab_glyph::FontArc>>,
    pub sections: Vec<TextMeasureSection>,
    pub text_alignment: TextAlignment,
    pub linebreak_behaviour: glyph_brush_layout::BuiltInLineBreaker,
    pub min_width_content_size: Vec2,
    pub max_width_content_size: Vec2,
}

impl TextMeasureInfo {
    fn new(
        fonts: Vec<ab_glyph::FontArc>,
        scaled_fonts: Vec<ab_glyph::PxScaleFont<ab_glyph::FontArc>>,
        sections: Vec<TextMeasureSection>,
        text_alignment: TextAlignment,
        linebreak_behaviour: glyph_brush_layout::BuiltInLineBreaker,
    ) -> Self {
        let mut info = Self {
            fonts,
            scaled_fonts,
            sections,
            text_alignment,
            linebreak_behaviour,
            min_width_content_size: Vec2::ZERO,
            max_width_content_size: Vec2::ZERO,
        };

        let section_texts = info.prepare_section_texts();
        let min =
            info.compute_size_from_section_texts(&section_texts, Vec2::new(0.0, f32::INFINITY));
        let max = info.compute_size_from_section_texts(
            &section_texts,
            Vec2::new(f32::INFINITY, f32::INFINITY),
        );
        info.min_width_content_size = min;
        info.max_width_content_size = max;
        info
    }

    fn prepare_section_texts(&self) -> Vec<SectionText> {
        self.sections
            .iter()
            .map(|section| SectionText {
                font_id: section.font_id,
                scale: section.scale,
                text: &section.text,
            })
            .collect::<Vec<_>>()
    }

    fn compute_size_from_section_texts(&self, sections: &[SectionText], bounds: Vec2) -> Vec2 {
        let geom = SectionGeometry {
            bounds: (bounds.x, bounds.y),
            ..Default::default()
        };
        let section_glyphs = glyph_brush_layout::Layout::default()
            .h_align(self.text_alignment.into())
            .line_breaker(self.linebreak_behaviour)
            .calculate_glyphs(&self.fonts, &geom, sections);

        compute_text_bounds(&section_glyphs, |index| &self.scaled_fonts[index]).size()
    }

    pub fn compute_size(&self, bounds: Vec2) -> Vec2 {
        let sections = self.prepare_section_texts();
        self.compute_size_from_section_texts(&sections, bounds)
    }
}
