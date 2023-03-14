use ab_glyph::{PxScale, ScaleFont};
use bevy_asset::{Assets, Handle, HandleId};
use bevy_ecs::component::Component;
use bevy_ecs::system::Resource;
use bevy_math::Vec2;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_utils::HashMap;

use glyph_brush_layout::{FontId, SectionText, SectionGeometry, GlyphPositioner};

use crate::{
    error::TextError, glyph_brush::GlyphBrush, scale_value, BreakLineOn, Font, FontAtlasSet,
    FontAtlasWarning, PositionedGlyph, TextAlignment, TextSection, TextSettings, YAxisOrientation,
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
        linebreak_behaviour: BreakLineOn,
        bounds: Vec2,
        font_atlas_set_storage: &mut Assets<FontAtlasSet>,
        texture_atlases: &mut Assets<TextureAtlas>,
        textures: &mut Assets<Image>,
        text_settings: &TextSettings,
        font_atlas_warning: &mut FontAtlasWarning,
        y_axis_orientation: YAxisOrientation,
    ) -> Result<TextLayoutInfo, TextError> {
        let mut scaled_fonts = Vec::new();
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
                .compute_glyphs(&sections, bounds, text_alignment, linebreak_behaviour)?;

        if section_glyphs.is_empty() {
            return Ok(TextLayoutInfo::default());
        }

        let mut min_x: f32 = std::f32::MAX;
        let mut min_y: f32 = std::f32::MAX;
        let mut max_x: f32 = std::f32::MIN;
        let mut max_y: f32 = std::f32::MIN;

        for sg in &section_glyphs {
            let scaled_font = scaled_fonts[sg.section_index];
            let glyph = &sg.glyph;
            min_x = min_x.min(glyph.position.x);
            min_y = min_y.min(glyph.position.y - scaled_font.ascent());
            max_x = max_x.max(glyph.position.x + scaled_font.h_advance(glyph.id));
            max_y = max_y.max(glyph.position.y - scaled_font.descent());
        }

        let size = Vec2::new(max_x - min_x, max_y - min_y);

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

    // hacked up `queue_text` that returns the text size constraints, there should be a better solution
    pub fn compute_size_constraints(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        scale_factor: f64,
        text_alignment: TextAlignment,
        linebreak_behaviour: BreakLineOn,
    ) -> Result<[Vec2; 2], TextError> {
        let mut scaled_fonts = Vec::new();
        let sections = sections
            .iter()
            .map(|section| {
                let font = fonts
                    .get(&section.style.font)
                    .ok_or(TextError::NoSuchFont)?;
                let font_id = self.get_or_insert_font_id(&section.style.font, font);
                let font_size = scale_value(section.style.font_size, scale_factor);

                let px_scale_font = ab_glyph::Font::into_scaled(font.font.clone(), font_size);
                scaled_fonts.push(px_scale_font);

                let section = SectionText {
                    font_id,
                    scale: PxScale::from(font_size),
                    text: &section.value,
                };

                Ok(section)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let result = [
            Vec2::new(0.0, f32::INFINITY),
            Vec2::new(f32::INFINITY, f32::INFINITY),
        ]
        .map(|bounds| {
            if let Ok(section_glyphs) =
                self.brush
                    .compute_glyphs(&sections, bounds, text_alignment, linebreak_behaviour)
            {
                let mut min_x: f32 = std::f32::MAX;
                let mut min_y: f32 = std::f32::MAX;
                let mut max_x: f32 = std::f32::MIN;
                let mut max_y: f32 = std::f32::MIN;

                for sg in section_glyphs {
                    let scaled_font = &scaled_fonts[sg.section_index];
                    let glyph = &sg.glyph;
                    min_x = min_x.min(glyph.position.x);
                    min_y = min_y.min(glyph.position.y - scaled_font.ascent());
                    max_x = max_x.max(glyph.position.x + scaled_font.h_advance(glyph.id));
                    max_y = max_y.max(glyph.position.y - scaled_font.descent());
                }

                Vec2::new(max_x - min_x, max_y - min_y)
            } else {
                Vec2::ZERO
            }
        });
        Ok(result)
    }

    pub fn compute_auto_text_measure(
        &mut self,
        fonts: &Assets<Font>,
        sections: &[TextSection],
        scale_factor: f64,
        text_alignment: TextAlignment,
        linebreak_behaviour: BreakLineOn,
    ) -> Result<AutoTextInfo, TextError> {
        let mut auto_fonts = vec![];
        let mut scaled_fonts = Vec::new();
        let sections = sections
            .iter()
            .enumerate()
            .map(|(i, section)| {
                let font = fonts
                    .get(&section.style.font)
                    .ok_or(TextError::NoSuchFont)?;
                let font_id = self.get_or_insert_font_id(&section.style.font, font);
                let font_size = scale_value(section.style.font_size, scale_factor);
                auto_fonts.push(font.font.clone());
                let px_scale_font = ab_glyph::Font::into_scaled(font.font.clone(), font_size);
                scaled_fonts.push(px_scale_font);

                let section = AutoTextSection {
                    font_id: FontId(i),
                    scale: PxScale::from(font_size),
                    text: section.value.clone(),
                };

                Ok(section)
            })
            .collect::<Result<Vec<_>, _>>()?;
        
        Ok(AutoTextInfo {
            fonts: auto_fonts,
            scaled_fonts,
            sections,
            text_alignment,
            linebreak_behaviour: linebreak_behaviour.into(),
        })

    }
}

#[derive(Clone)]
pub struct AutoTextSection {
    pub text: String, 
    pub scale: PxScale,
    pub font_id: FontId,
}

#[derive(Clone)]
pub struct AutoTextInfo {
    pub fonts: Vec<ab_glyph::FontArc>,
    pub scaled_fonts: Vec<ab_glyph::PxScaleFont<ab_glyph::FontArc>>,
    pub sections: Vec<AutoTextSection>,
    pub text_alignment: TextAlignment,
    pub linebreak_behaviour: glyph_brush_layout::BuiltInLineBreaker,
}

impl AutoTextInfo {
    pub fn compute_size(
        &self,
        bounds: Vec2,
    ) -> Vec2 {
        let geom = SectionGeometry {
            bounds: (bounds.x, bounds.y),
            ..Default::default()
        };

        let sections = self.sections.iter().map(|section| {
            SectionText {
                font_id: section.font_id,
                scale: section.scale,
                text: &section.text,
            }
        }).collect::<Vec<_>>();

        let section_glyphs = glyph_brush_layout::Layout::default()
            .h_align(self.text_alignment.into())
            .line_breaker(self.linebreak_behaviour)
            .calculate_glyphs(&self.fonts, &geom, &sections);

        let mut min_x: f32 = std::f32::MAX;
        let mut min_y: f32 = std::f32::MAX;
        let mut max_x: f32 = std::f32::MIN;
        let mut max_y: f32 = std::f32::MIN;

        for sg in section_glyphs {
            let scaled_font = &self.scaled_fonts[sg.section_index];
            let glyph = &sg.glyph;
            min_x = min_x.min(glyph.position.x);
            min_y = min_y.min(glyph.position.y - scaled_font.ascent());
            max_x = max_x.max(glyph.position.x + scaled_font.h_advance(glyph.id));
            max_y = max_y.max(glyph.position.y - scaled_font.descent());
        }
        Vec2::new(max_x - min_x, max_y - min_y)
    }
}