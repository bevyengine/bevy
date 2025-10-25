use crate::add_glyph_to_atlas;
use crate::get_glyph_atlas_info;
use crate::FontAtlasKey;
use crate::FontAtlasSet;
use crate::FontSmoothing;
use crate::GlyphCacheKey;
use crate::RunGeometry;
use crate::TextLayoutInfo;
use bevy_asset::Assets;
use bevy_image::Image;
use bevy_image::TextureAtlasLayout;
use bevy_math::Rect;
use bevy_math::UVec2;
use bevy_math::Vec2;
use parley::swash::FontRef;
use parley::Alignment;
use parley::AlignmentOptions;
use parley::Brush;
use parley::FontContext;
use parley::FontStack;
use parley::Layout;
use parley::LayoutContext;
use parley::PositionedLayoutItem;
use parley::StyleProperty;
use parley::WordBreakStrength;
use std::ops::Range;
use std::usize;
use swash::scale::ScaleContext;

fn concat_text_for_layout<'a>(
    text_sections: impl Iterator<Item = &'a str>,
) -> (String, Vec<Range<usize>>) {
    let mut out = String::new();
    let mut ranges = Vec::new();

    for text_section in text_sections {
        let start = out.len();
        out.push_str(text_section);
        let end = out.len();
        ranges.push(start..end);
    }

    (out, ranges)
}

/// Resolved text style
#[derive(Clone, Copy, Debug)]
pub struct TextSectionStyle<'a, B> {
    font_family: &'a str,
    font_size: f32,
    line_height: crate::text::LineHeight,
    brush: B,
}

impl<'a, B: Brush> TextSectionStyle<'a, B> {
    /// new text section style
    pub fn new(family: &'a str, size: f32, line_height: crate::LineHeight, brush: B) -> Self {
        Self {
            font_family: family,
            font_size: size,
            line_height,
            brush,
        }
    }
}

/// Create layout given text sections and styles
pub fn shape_text_from_sections<'a, B: Brush>(
    layout: &mut Layout<B>,
    font_cx: &'a mut FontContext,
    layout_cx: &'a mut LayoutContext<B>,
    text_sections: impl Iterator<Item = &'a str>,
    text_section_styles: impl Iterator<Item = TextSectionStyle<'a, B>>,
    scale_factor: f32,
    line_break: crate::text::LineBreak,
) {
    let (text, section_ranges) = concat_text_for_layout(text_sections);
    let mut builder = layout_cx.ranged_builder(font_cx, &text, scale_factor, true);
    if let Some(word_break_strength) = match line_break {
        crate::LineBreak::WordBoundary => Some(WordBreakStrength::Normal),
        crate::LineBreak::AnyCharacter => Some(WordBreakStrength::BreakAll),
        crate::LineBreak::WordOrCharacter => Some(WordBreakStrength::KeepAll),
        _ => None,
    } {
        builder.push_default(StyleProperty::WordBreak(word_break_strength));
    };
    for (style, range) in text_section_styles.zip(section_ranges) {
        builder.push(StyleProperty::Brush(style.brush), range.clone());
        builder.push(FontStack::from(style.font_family), range.clone());
        builder.push(StyleProperty::FontSize(style.font_size), range.clone());
        builder.push(style.line_height.eval(), range);
    }
    builder.build_into(layout, &text);
}

/// create a TextLayoutInfo
pub fn update_text_layout_info(
    layout: &mut Layout<u32>,
    max_advance: Option<f32>,
    alignment: Alignment,
    scale_cx: &mut ScaleContext,
    font_atlas_set: &mut FontAtlasSet,
    texture_atlases: &mut Assets<TextureAtlasLayout>,
    textures: &mut Assets<Image>,
    font_smoothing: FontSmoothing,
) -> TextLayoutInfo {
    layout.break_all_lines(max_advance);
    layout.align(None, alignment, AlignmentOptions::default());

    let mut info = TextLayoutInfo::default();

    info.scale_factor = layout.scale();
    info.size = (
        layout.width() / layout.scale(),
        layout.height() / layout.scale(),
    )
        .into();

    for line in layout.lines() {
        for (line_index, item) in line.items().enumerate() {
            match item {
                PositionedLayoutItem::GlyphRun(glyph_run) => {
                    let span_index = glyph_run.style().brush;

                    let run = glyph_run.run();

                    let font = run.font();
                    let font_size = run.font_size();
                    let coords = run.normalized_coords();

                    let font_atlas_key = FontAtlasKey::new(&font, font_size, font_smoothing);

                    for glyph in glyph_run.positioned_glyphs() {
                        let font_atlases = font_atlas_set.entry(font_atlas_key).or_default();
                        let Ok(atlas_info) = get_glyph_atlas_info(
                            font_atlases,
                            GlyphCacheKey {
                                glyph_id: glyph.id as u16,
                            },
                        )
                        .map(Ok)
                        .unwrap_or_else(|| {
                            let font_ref =
                                FontRef::from_index(font.data.as_ref(), font.index as usize)
                                    .unwrap();
                            let mut scaler = scale_cx
                                .builder(font_ref)
                                .size(font_size)
                                .hint(true)
                                .normalized_coords(coords)
                                .build();
                            add_glyph_to_atlas(
                                font_atlases,
                                texture_atlases,
                                textures,
                                &mut scaler,
                                font_smoothing,
                                glyph.id as u16,
                            )
                        }) else {
                            continue;
                        };

                        let texture_atlas = texture_atlases.get(atlas_info.texture_atlas).unwrap();
                        let location = atlas_info.location;
                        let glyph_rect = texture_atlas.textures[location.glyph_index];
                        let glyph_size = UVec2::new(glyph_rect.width(), glyph_rect.height());
                        let x = glyph_size.x as f32 / 2. + glyph.x + location.offset.x as f32;
                        let y = glyph_size.y as f32 / 2. + glyph.y - location.offset.y as f32;

                        info.glyphs.push(crate::PositionedGlyph {
                            position: (x, y).into(),
                            size: glyph_size.as_vec2(),
                            atlas_info,
                            span_index: span_index as usize,
                            line_index,
                            byte_index: line.text_range().start,
                            byte_length: line.text_range().len(),
                        });
                    }

                    info.run_geometry.push(RunGeometry {
                        span_index: span_index as usize,
                        bounds: Rect {
                            min: Vec2::new(glyph_run.offset(), line.metrics().min_coord),
                            max: Vec2::new(
                                glyph_run.offset() + glyph_run.advance(),
                                line.metrics().max_coord,
                            ),
                        },
                        strikethrough_y: glyph_run.baseline() - run.metrics().strikethrough_offset,
                        strikethrough_thickness: run.metrics().strikethrough_size,
                        underline_y: glyph_run.baseline() - run.metrics().underline_offset,
                        underline_thickness: run.metrics().underline_size,
                    });
                }
                _ => {}
            }
        }
    }

    info
}
