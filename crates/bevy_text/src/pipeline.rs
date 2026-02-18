use alloc::borrow::Cow;

use core::hash::BuildHasher;

use bevy_asset::Assets;
use bevy_color::Color;
use bevy_ecs::{
    component::Component, entity::Entity, reflect::ReflectComponent, resource::Resource,
    system::ResMut,
};
use bevy_image::prelude::*;
use bevy_log::warn_once;
use bevy_math::{Rect, Vec2};
use bevy_platform::hash::FixedHasher;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use parley::style::{OverflowWrap, TextWrapMode};
use parley::{
    Alignment, AlignmentOptions, FontFamily, FontStack, Layout, PositionedLayoutItem,
    StyleProperty, WordBreakStrength,
};
use swash::FontRef;

use crate::{
    add_glyph_to_atlas,
    error::TextError,
    get_glyph_atlas_info,
    parley_context::{FontCx, LayoutCx, ScaleCx},
    ComputedTextBlock, Font, FontAtlasKey, FontAtlasSet, FontHinting, FontSmoothing, FontSource,
    Justify, LineBreak, LineHeight, PositionedGlyph, TextBounds, TextEntity, TextFont, TextLayout,
};

/// The `TextPipeline` is used to layout and render text blocks (see `Text`/`Text2d`).
#[derive(Resource, Default)]
pub struct TextPipeline {
    /// Buffered vec for collecting text sections.
    ///
    /// See <https://users.rust-lang.org/t/how-to-cache-a-vectors-capacity/94478/10>.
    sections_buffer: Vec<(usize, &'static str, &'static TextFont, f32, LineHeight)>,
    /// Buffered string for concatenated text content.
    text_buffer: String,
}

impl TextPipeline {
    /// Shapes and lays out text spans into the computed buffer.
    ///
    /// Negative or 0.0 font sizes will not be laid out.
    pub fn update_buffer<'a>(
        &mut self,
        fonts: &Assets<Font>,
        text_spans: impl Iterator<Item = (Entity, usize, &'a str, &'a TextFont, Color, LineHeight)>,
        linebreak: LineBreak,
        justify: Justify,
        bounds: TextBounds,
        scale_factor: f32,
        computed: &mut ComputedTextBlock,
        font_system: &mut FontCx,
        layout_cx: &mut LayoutCx,
        logical_viewport_size: Vec2,
        base_rem_size: f32,
    ) -> Result<(), TextError> {
        computed.entities.clear();
        computed.needs_rerender = false;
        computed.uses_rem_sizes = false;
        computed.uses_viewport_sizes = false;

        if scale_factor <= 0.0 {
            warn_once!("Text scale factor is <= 0.0. No text will be displayed.");
            return Err(TextError::DegenerateScaleFactor);
        }

        let mut sections: Vec<(usize, &str, &TextFont, f32, LineHeight)> =
            core::mem::take(&mut self.sections_buffer)
                .into_iter()
                .map(|_| -> (usize, &str, &TextFont, f32, LineHeight) { unreachable!() })
                .collect();

        let result = {
            for (span_index, (entity, depth, span, text_font, _color, line_height)) in
                text_spans.enumerate()
            {
                match text_font.font_size {
                    crate::FontSize::Vw(_)
                    | crate::FontSize::Vh(_)
                    | crate::FontSize::VMin(_)
                    | crate::FontSize::VMax(_) => computed.uses_viewport_sizes = true,
                    crate::FontSize::Rem(_) => computed.uses_rem_sizes = true,
                    _ => (),
                }

                computed.entities.push(TextEntity {
                    entity,
                    depth,
                    font_smoothing: text_font.font_smoothing,
                });

                if span.is_empty() {
                    continue;
                }

                if matches!(text_font.font, FontSource::Handle(_))
                    && resolve_font_source(&text_font.font, fonts).is_err()
                {
                    return Err(TextError::NoSuchFont);
                }

                let font_size = text_font
                    .font_size
                    .eval(logical_viewport_size, base_rem_size);

                if font_size <= 0.0 {
                    warn_once!(
                        "Text span {entity} has a font size <= 0.0. Nothing will be displayed."
                    );
                    continue;
                }

                const WARN_FONT_SIZE: f32 = 1000.0;
                if font_size > WARN_FONT_SIZE {
                    warn_once!(
                        "Text span {entity} has an excessively large font size ({} with scale factor {}). \
                        Extremely large font sizes will cause performance issues with font atlas \
                        generation and high memory usage.",
                        font_size,
                        scale_factor,
                    );
                }

                sections.push((span_index, span, text_font, font_size, line_height));
            }

            self.text_buffer.clear();
            for (_, span, _, _, _) in &sections {
                self.text_buffer.push_str(span);
            }

            let text = self.text_buffer.as_str();
            let layout = &mut computed.layout;
            let mut builder =
                layout_cx
                    .0
                    .ranged_builder(&mut font_system.0, text, scale_factor, true);

            match linebreak {
                LineBreak::AnyCharacter => {
                    builder.push_default(StyleProperty::WordBreak(WordBreakStrength::BreakAll));
                }
                LineBreak::WordOrCharacter => {
                    builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
                }
                LineBreak::NoWrap => {
                    builder.push_default(StyleProperty::TextWrapMode(TextWrapMode::NoWrap));
                }
                LineBreak::WordBoundary => {
                    builder.push_default(StyleProperty::WordBreak(WordBreakStrength::Normal));
                }
            }

            let mut start = 0;
            for (span_index, span, text_font, font_size, line_height) in sections.drain(..) {
                let end = start + span.len();
                let range = start..end;
                start = end;

                if range.is_empty() {
                    continue;
                }

                let family = resolve_font_source(&text_font.font, fonts)?;

                builder.push(
                    StyleProperty::FontStack(FontStack::Single(family)),
                    range.clone(),
                );
                builder.push(
                    StyleProperty::Brush((span_index as u32, text_font.font_smoothing)),
                    range.clone(),
                );
                builder.push(StyleProperty::FontSize(font_size), range.clone());
                builder.push(
                    StyleProperty::LineHeight(line_height.eval(font_size)),
                    range.clone(),
                );
                builder.push(
                    StyleProperty::FontWeight(text_font.weight.into()),
                    range.clone(),
                );
                builder.push(
                    StyleProperty::FontWidth(text_font.width.into()),
                    range.clone(),
                );
                builder.push(
                    StyleProperty::FontStyle(text_font.style.into()),
                    range.clone(),
                );
                builder.push(
                    StyleProperty::FontFeatures((&text_font.font_features).into()),
                    range,
                );
            }

            builder.build_into(layout, text);
            layout_with_bounds(layout, bounds, justify);
            Ok(())
        };

        sections.clear();
        self.sections_buffer = sections
            .into_iter()
            .map(
                |_| -> (usize, &'static str, &'static TextFont, f32, LineHeight) { unreachable!() },
            )
            .collect();

        result
    }

    /// Queues text for measurement.
    pub fn create_text_measure<'a>(
        &mut self,
        entity: Entity,
        fonts: &Assets<Font>,
        text_spans: impl Iterator<Item = (Entity, usize, &'a str, &'a TextFont, Color, LineHeight)>,
        scale_factor: f32,
        layout: &TextLayout,
        computed: &mut ComputedTextBlock,
        font_system: &mut FontCx,
        layout_cx: &mut LayoutCx,
        logical_viewport_size: Vec2,
        base_rem_size: f32,
    ) -> Result<TextMeasureInfo, TextError> {
        const MIN_WIDTH_CONTENT_BOUNDS: TextBounds = TextBounds::new_horizontal(0.0);

        computed.needs_rerender = false;

        self.update_buffer(
            fonts,
            text_spans,
            layout.linebreak,
            layout.justify,
            MIN_WIDTH_CONTENT_BOUNDS,
            scale_factor,
            computed,
            font_system,
            layout_cx,
            logical_viewport_size,
            base_rem_size,
        )?;

        let layout_buffer = &mut computed.layout;
        let min_width_content_size = buffer_dimensions(layout_buffer);

        layout_with_bounds(layout_buffer, TextBounds::UNBOUNDED, layout.justify);
        let max_width_content_size = buffer_dimensions(layout_buffer);

        Ok(TextMeasureInfo {
            min: min_width_content_size,
            max: max_width_content_size,
            entity,
        })
    }

    /// Update [`TextLayoutInfo`] with the new [`PositionedGlyph`] layout.
    pub fn update_text_layout_info(
        &mut self,
        layout_info: &mut TextLayoutInfo,
        font_atlas_set: &mut FontAtlasSet,
        textures: &mut Assets<Image>,
        computed: &mut ComputedTextBlock,
        scale_cx: &mut ScaleCx,
        bounds: TextBounds,
        justify: Justify,
        hinting: FontHinting,
    ) -> Result<(), TextError> {
        computed.needs_rerender = false;
        layout_info.clear();

        let layout = &mut computed.layout;
        layout_with_bounds(layout, bounds, justify);

        for (line_index, line) in layout.lines().enumerate() {
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                    let span_index = glyph_run.style().brush.0 as usize;
                    let font_smoothing = glyph_run.style().brush.1;
                    let run = glyph_run.run();
                    let font = run.font();
                    let font_size = run.font_size();
                    let coords = run.normalized_coords();
                    let variations_hash = FixedHasher.hash_one(coords);
                    let text_range = run.text_range();
                    let font_atlas_key = FontAtlasKey {
                        id: font.data.id() as u32,
                        index: font.index,
                        font_size_bits: font_size.to_bits(),
                        variations_hash,
                        hinting,
                        font_smoothing,
                    };

                    let Some(font_ref) =
                        FontRef::from_index(font.data.as_ref(), font.index as usize)
                    else {
                        return Err(TextError::NoSuchFont);
                    };

                    let hint =
                        hinting.should_hint() && font_smoothing == FontSmoothing::AntiAliased;
                    let mut scaler = scale_cx
                        .0
                        .builder(font_ref)
                        .size(font_size)
                        .hint(hint)
                        .normalized_coords(coords)
                        .build();

                    for glyph in glyph_run.positioned_glyphs() {
                        let Ok(glyph_id) = u16::try_from(glyph.id) else {
                            continue;
                        };

                        let font_atlases = font_atlas_set.entry(font_atlas_key).or_default();
                        let atlas_info =
                            get_glyph_atlas_info(font_atlases, crate::GlyphCacheKey { glyph_id })
                                .map(Ok)
                                .unwrap_or_else(|| {
                                    add_glyph_to_atlas(
                                        font_atlases,
                                        textures,
                                        &mut scaler,
                                        font_smoothing,
                                        glyph_id,
                                    )
                                })?;

                        let glyph_pos = Vec2::new(glyph.x, glyph.y);
                        let size = atlas_info.rect.size();

                        layout_info.glyphs.push(PositionedGlyph {
                            position: size / 2.
                                + if font_smoothing == FontSmoothing::None {
                                    glyph_pos.floor()
                                } else {
                                    glyph_pos
                                }
                                + atlas_info.offset,
                            atlas_info,
                            span_index,
                            byte_index: text_range.start,
                            byte_length: text_range.len(),
                            line_index,
                        });
                    }

                    layout_info.run_geometry.push(RunGeometry {
                        span_index,
                        bounds: Rect::new(
                            glyph_run.offset(),
                            line.metrics().min_coord,
                            glyph_run.offset() + glyph_run.advance(),
                            line.metrics().max_coord,
                        ),
                        strikethrough_y: glyph_run.baseline() - run.metrics().strikethrough_offset,
                        strikethrough_thickness: run.metrics().strikethrough_size,
                        underline_y: glyph_run.baseline() - run.metrics().underline_offset,
                        underline_thickness: run.metrics().underline_size,
                    });
                }
            }
        }

        layout_info.size = Vec2::new(layout.full_width(), layout.height()).ceil();
        Ok(())
    }
}

fn resolve_font_source<'a>(
    font: &'a FontSource,
    fonts: &'a Assets<Font>,
) -> Result<FontFamily<'a>, TextError> {
    Ok(match font {
        FontSource::Handle(handle) => {
            let font = fonts.get(handle.id()).ok_or(TextError::NoSuchFont)?;
            FontFamily::Named(Cow::Borrowed(font.family_name.as_str()))
        }
        FontSource::Family(family) => FontFamily::Named(Cow::Borrowed(family.as_str())),
        FontSource::Serif => FontFamily::Generic(parley::GenericFamily::Serif),
        FontSource::SansSerif => FontFamily::Generic(parley::GenericFamily::SansSerif),
        FontSource::Cursive => FontFamily::Generic(parley::GenericFamily::Cursive),
        FontSource::Fantasy => FontFamily::Generic(parley::GenericFamily::Fantasy),
        FontSource::Monospace => FontFamily::Generic(parley::GenericFamily::Monospace),
        FontSource::SystemUi => FontFamily::Generic(parley::GenericFamily::SystemUi),
        FontSource::UiSerif => FontFamily::Generic(parley::GenericFamily::UiSerif),
        FontSource::UiSansSerif => FontFamily::Generic(parley::GenericFamily::UiSansSerif),
        FontSource::UiMonospace => FontFamily::Generic(parley::GenericFamily::UiMonospace),
        FontSource::UiRounded => FontFamily::Generic(parley::GenericFamily::UiRounded),
        FontSource::Emoji => FontFamily::Generic(parley::GenericFamily::Emoji),
        FontSource::Math => FontFamily::Generic(parley::GenericFamily::Math),
        FontSource::FangSong => FontFamily::Generic(parley::GenericFamily::FangSong),
    })
}

/// Render information for a corresponding text block.
///
/// Contains scaled glyphs and their size. Generated via [`TextPipeline::update_text_layout_info`] when an entity has
/// [`TextLayout`] and [`ComputedTextBlock`] components.
#[derive(Component, Clone, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct TextLayoutInfo {
    /// The target scale factor for this text layout
    pub scale_factor: f32,
    /// Scaled and positioned glyphs in screenspace
    pub glyphs: Vec<PositionedGlyph>,
    /// Geometry of each text run used to render text decorations like background colors, strikethrough, and underline.
    /// A run in `bevy_text` is a contiguous sequence of glyphs on a line that share the same text attributes like font,
    /// font size, and line height. A text entity that extends over multiple lines will have multiple corresponding runs.
    ///
    /// The coordinates are unscaled and relative to the top left corner of the text layout.
    pub run_geometry: Vec<RunGeometry>,
    /// The glyphs resulting size
    pub size: Vec2,
}

impl TextLayoutInfo {
    /// Clear the layout, retaining capacity
    pub fn clear(&mut self) {
        self.scale_factor = 1.;
        self.glyphs.clear();
        self.run_geometry.clear();
        self.size = Vec2::ZERO;
    }
}

/// Geometry of a text run used to render text decorations like background colors, strikethrough, and underline.
/// A run in `bevy_text` is a contiguous sequence of glyphs on a line that share the same text attributes like font,
/// font size, and line height.
#[derive(Default, Debug, Clone, Reflect)]
pub struct RunGeometry {
    /// The index of the text entity in [`ComputedTextBlock`] that this run belongs to.
    pub span_index: usize,
    /// Bounding box around the text run.
    pub bounds: Rect,
    /// Y position of the strikethrough in the text layout.
    pub strikethrough_y: f32,
    /// Strikethrough stroke thickness.
    pub strikethrough_thickness: f32,
    /// Y position of the underline in the text layout.
    pub underline_y: f32,
    /// Underline stroke thickness.
    pub underline_thickness: f32,
}

impl RunGeometry {
    /// Returns the center of the strikethrough in the text layout.
    pub fn strikethrough_position(&self) -> Vec2 {
        Vec2::new(
            self.bounds.center().x,
            self.strikethrough_y + 0.5 * self.strikethrough_thickness,
        )
    }

    /// Returns the size of the strikethrough.
    pub fn strikethrough_size(&self) -> Vec2 {
        Vec2::new(self.bounds.size().x, self.strikethrough_thickness)
    }

    /// Returns the center of the underline in the text layout.
    pub fn underline_position(&self) -> Vec2 {
        Vec2::new(
            self.bounds.center().x,
            self.underline_y + 0.5 * self.underline_thickness,
        )
    }

    /// Returns the size of the underline.
    pub fn underline_size(&self) -> Vec2 {
        Vec2::new(self.bounds.size().x, self.underline_thickness)
    }
}

/// Size information for a corresponding [`ComputedTextBlock`] component.
///
/// Generated via [`TextPipeline::create_text_measure`].
#[derive(Debug)]
pub struct TextMeasureInfo {
    /// Minimum size for a text area in pixels, to be used when laying out widgets with taffy.
    pub min: Vec2,
    /// Maximum size for a text area in pixels, to be used when laying out widgets with taffy.
    pub max: Vec2,
    /// The entity that is measured.
    pub entity: Entity,
}

impl TextMeasureInfo {
    /// Computes the size of the text area within the provided bounds.
    pub fn compute_size(
        &mut self,
        bounds: TextBounds,
        computed: &mut ComputedTextBlock,
        _font_system: &mut FontCx,
    ) -> Vec2 {
        // Note that this arbitrarily adjusts the buffer layout. We assume the buffer is always 'refreshed'
        // whenever a canonical state is required.
        let layout = &mut computed.layout;
        layout.break_all_lines(bounds.width);
        layout.align(bounds.width, Alignment::Start, AlignmentOptions::default());
        buffer_dimensions(layout)
    }
}

fn layout_with_bounds(
    layout: &mut Layout<(u32, FontSmoothing)>,
    bounds: TextBounds,
    justify: Justify,
) {
    layout.break_all_lines(bounds.width);

    let container_width = if bounds.width.is_none() && justify != Justify::Left {
        Some(layout.width())
    } else {
        bounds.width
    };

    layout.align(container_width, justify.into(), AlignmentOptions::default());
}

/// Calculate the size of the text area for the given buffer.
fn buffer_dimensions(buffer: &Layout<(u32, FontSmoothing)>) -> Vec2 {
    let size = Vec2::new(buffer.full_width(), buffer.height());
    if size.is_finite() {
        size.ceil()
    } else {
        Vec2::ZERO
    }
}

/// Discards stale data cached in the font system.
pub(crate) fn trim_source_cache(mut font_cx: ResMut<FontCx>) {
    // A trim age of 2 was found to reduce frame time variance vs age of 1 when tested with dynamic text.
    // See https://github.com/bevyengine/bevy/pull/15037
    //
    // We assume only text updated frequently benefits from the shape cache (e.g. animated text, or
    // text that is dynamically measured for UI).
    font_cx.0.source_cache.prune(2, false);
}
