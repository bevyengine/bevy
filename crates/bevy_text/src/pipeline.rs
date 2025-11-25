use crate::add_glyph_to_atlas;
use crate::get_glyph_atlas_info;
use crate::Font;
use crate::FontAtlasKey;
use crate::FontAtlasSet;
use crate::FontSmoothing;
use crate::GlyphCacheKey;

use crate::RunGeometry;
use crate::TextEntity;
use crate::TextFont;
use crate::TextHead;
use crate::TextLayoutInfo;
use crate::TextReader;
use bevy_asset::Assets;
use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use bevy_image::Image;
use bevy_image::TextureAtlasLayout;
use bevy_math::Rect;
use bevy_math::UVec2;
use bevy_math::Vec2;
use parley::swash::FontRef;
use parley::Alignment;
use parley::AlignmentOptions;
use parley::FontContext;
use parley::FontSettings;
use parley::FontStack;
use parley::Layout;
use parley::LayoutContext;
use parley::LineHeight;
use parley::PositionedLayoutItem;
use parley::StyleProperty;
use parley::WordBreakStrength;
use smallvec::SmallVec;
use std::usize;
use swash::scale::ScaleContext;

/// The `TextPipeline` is used to layout and render text blocks (see `Text`/`Text2d`).
///
/// See the [crate-level documentation](crate) for more information.
#[derive(Default, Resource)]
pub struct TextPipeline {
    /// Buffered vec for collecting spans.
    ///
    /// See [this dark magic](https://users.rust-lang.org/t/how-to-cache-a-vectors-capacity/94478/10).
    spans_buffer: Vec<(&'static str, &'static TextFont, LineHeight)>,
}

impl TextPipeline {
    /// Create layout given text sections and styles
    pub fn shape_text<'a, T: TextHead>(
        &mut self,
        text_root_entity: Entity,
        reader: &mut TextReader<T>,
        layout: &mut Layout<u32>,
        font_cx: &'a mut FontContext,
        layout_cx: &'a mut LayoutContext<u32>,
        scale_factor: f32,
        line_break: crate::text::LineBreak,
        fonts: &Assets<Font>,
        entities: &mut SmallVec<[TextEntity; 1]>,
    ) {
        entities.clear();

        let mut spans: Vec<(&str, &TextFont, LineHeight)> = core::mem::take(&mut self.spans_buffer)
            .into_iter()
            .map(|_| -> (&str, &TextFont, LineHeight) { unreachable!() })
            .collect();

        let mut text_len = 0;
        for (entity, depth, text_section, text_font, _, line_height) in
            reader.iter(text_root_entity)
        {
            entities.push(TextEntity { entity, depth });
            text_len += text_section.len();
            spans.push((text_section, text_font, line_height.eval()));
        }

        let mut text = String::with_capacity(text_len);
        for (text_section, ..) in &spans {
            text.push_str(*text_section);
        }

        let mut builder = layout_cx.ranged_builder(font_cx, &text, scale_factor, true);
        if let Some(word_break_strength) = match line_break {
            crate::LineBreak::WordBoundary => Some(WordBreakStrength::Normal),
            crate::LineBreak::AnyCharacter => Some(WordBreakStrength::BreakAll),
            crate::LineBreak::WordOrCharacter => Some(WordBreakStrength::KeepAll),
            _ => None,
        } {
            builder.push_default(StyleProperty::WordBreak(word_break_strength));
        };

        let mut start = 0;
        for (index, (text_section, text_font, line_height)) in spans.drain(..).enumerate() {
            let end = start + text_section.len();
            let range = start..end;
            start = end;
            if let Some(family) = fonts
                .get(text_font.font.id())
                .map(|font| font.family_name.as_str())
            {
                builder.push(FontStack::from(family), range.clone());
            };
            builder.push(StyleProperty::Brush(index as u32), range.clone());
            builder.push(StyleProperty::FontSize(text_font.font_size), range.clone());
            builder.push(line_height, range.clone());
            builder.push(
                StyleProperty::FontFeatures(FontSettings::from(text_font.font_features.as_slice())),
                range,
            );
        }
        builder.build_into(layout, &text);

        // Recover the spans buffer.
        self.spans_buffer = spans
            .into_iter()
            .map(|_| -> (&'static str, &'static TextFont, LineHeight) { unreachable!() })
            .collect();
    }
}

/// create a TextLayoutInfo
pub fn update_text_layout_info(
    info: &mut TextLayoutInfo,
    layout: &mut Layout<u32>,
    max_advance: Option<f32>,
    alignment: Alignment,
    scale_cx: &mut ScaleContext,
    font_atlas_set: &mut FontAtlasSet,
    texture_atlases: &mut Assets<TextureAtlasLayout>,
    textures: &mut Assets<Image>,
    font_smoothing: FontSmoothing,
) {
    info.clear();

    layout.break_all_lines(max_advance);
    layout.align(None, alignment, AlignmentOptions::default());

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
}
