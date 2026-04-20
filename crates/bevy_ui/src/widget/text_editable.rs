use core::hash::BuildHasher;
use core::time::Duration;

use crate::{ComputedNode, ComputedUiRenderTargetInfo, ContentSize, NodeMeasure};
use bevy_asset::Assets;

use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    query::Changed,
    system::{Local, Query, Res, ResMut},
    world::Ref,
};
use bevy_image::prelude::*;
use bevy_input_focus::InputFocus;
use bevy_math::{Rect, Vec2};
use bevy_platform::hash::FixedHasher;
use bevy_text::{
    add_glyph_to_atlas, get_glyph_atlas_info, resolve_font_source, EditableText,
    EditableTextGeneration, Font, FontAtlasKey, FontAtlasSet, FontCx, FontHinting, FontSize,
    GlyphCacheKey, LayoutCx, LineBreak, LineHeight, PositionedGlyph, RemSize, RunGeometry, ScaleCx,
    TextBrush, TextFont, TextLayout, TextLayoutInfo,
};
use bevy_time::{Real, Time};
use parley::{BoundingBox, PositionedLayoutItem, StyleProperty};
use swash::FontRef;
use taffy::MaybeMath;

#[derive(Component, Clone, Copy, PartialEq, Debug, Default)]
pub struct TextScroll(pub Vec2);

struct TextInputMeasure {
    width: Option<f32>,
    height: Option<f32>,
}

impl crate::Measure for TextInputMeasure {
    fn measure(&mut self, measure_args: crate::MeasureArgs<'_>) -> Vec2 {
        let width = measure_args.resolve_width();
        let height = measure_args.resolve_height();

        let x = width
            .effective
            .unwrap_or(self.width.unwrap_or(match measure_args.available_width {
                crate::AvailableSpace::Definite(x) => x,
                crate::AvailableSpace::MinContent | crate::AvailableSpace::MaxContent => 0.0,
            }))
            .maybe_clamp(width.min, width.max);
        let y = height
            .effective
            .unwrap_or(self.height.unwrap_or(match measure_args.available_height {
                crate::AvailableSpace::Definite(y) => y,
                crate::AvailableSpace::MinContent | crate::AvailableSpace::MaxContent => 0.0,
            }))
            .maybe_clamp(height.min, height.max);

        Vec2::new(x, y).ceil()
    }
}

/// If `visible_lines` or `visible_width` are `Some`, sets a `ContentSize` that determines:
/// - node height as `line_height * visible_lines`, using the resolved font line height.
/// - node width as `advance('0') * visible_width`, where `advance('0')` is looked up from font metrics.
pub fn update_editable_text_content_size(
    mut text_input_query: Query<(
        Ref<EditableText>,
        Ref<TextFont>,
        Ref<LineHeight>,
        Ref<ComputedUiRenderTargetInfo>,
        &mut ContentSize,
    )>,
    fonts: Res<Assets<Font>>,
    mut font_cx: ResMut<FontCx>,
    rem_size: Res<RemSize>,
) {
    for (editable_text, text_font, line_height, target, mut content_size) in &mut text_input_query {
        if !(editable_text.is_changed()
            || text_font.is_changed()
            || line_height.is_changed()
            || target.is_changed()
            || fonts.is_changed()
            || rem_size.is_changed())
        {
            continue;
        }

        let font_size = text_font.font_size.eval(target.logical_size(), rem_size.0);

        let width = editable_text.visible_width.and_then(|visible_width| {
            let font_context = &mut font_cx.0;
            let mut query = font_context
                .collection
                .query(&mut font_context.source_cache);
            match resolve_font_source(&text_font.font, fonts.as_ref()).ok()? {
                parley::FontFamily::Single(parley::FontFamilyName::Named(name)) => {
                    query.set_families([parley::fontique::QueryFamily::Named(name.as_ref())]);
                }
                parley::FontFamily::Single(parley::FontFamilyName::Generic(generic)) => {
                    query.set_families([parley::fontique::QueryFamily::Generic(generic)]);
                }
                _ => return None,
            }
            query.set_attributes(parley::fontique::Attributes::new(
                text_font.width.into(),
                text_font.style.into(),
                text_font.weight.into(),
            ));

            let mut width = None;
            query.matches_with(|query_font| {
                let Some((glyph_id, font_ref)) = query_font
                    .charmap()
                    .and_then(|char_map| char_map.map('0'))
                    .and_then(|glyph_id| u16::try_from(glyph_id).ok())
                    .zip(FontRef::from_index(
                        query_font.blob.as_ref(),
                        query_font.index as usize,
                    ))
                else {
                    return parley::fontique::QueryStatus::Continue;
                };

                let advance = font_ref
                    .glyph_metrics(&[])
                    .scale(font_size)
                    .advance_width(glyph_id);
                if advance.is_finite() {
                    width = Some(advance.max(0.0));
                    parley::fontique::QueryStatus::Stop
                } else {
                    parley::fontique::QueryStatus::Continue
                }
            });

            width.map(|width| width * visible_width * target.scale_factor())
        });

        let height = editable_text.visible_lines.map(|visible_lines| {
            let logical_line_height = match *line_height {
                LineHeight::Px(px) => px,
                LineHeight::RelativeToFont(scale) => scale * font_size,
            };
            visible_lines * logical_line_height * target.scale_factor()
        });

        if width.is_some() || height.is_some() {
            content_size.set(NodeMeasure::Custom(Box::new(TextInputMeasure {
                width,
                height,
            })));
        } else {
            content_size.clear();
        }
    }
}

/// Syncs each [`EditableText`] entity's [`PlainEditor`](parley::PlainEditor)
/// style properties to match its [`TextFont`], [`LineHeight`], and [`TextLayout`] components.
pub fn update_editable_text_styles(
    fonts: Res<Assets<Font>>,
    mut editable_text_query: Query<(
        &mut EditableText,
        Ref<TextFont>,
        Ref<LineHeight>,
        Ref<ComputedUiRenderTargetInfo>,
        Ref<TextLayout>,
    )>,
    rem_size: Res<RemSize>,
) {
    for (mut editable_text, text_font, line_height, target, text_layout) in
        editable_text_query.iter_mut()
    {
        let editor = editable_text.editor_mut();

        if f32::EPSILON < (target.scale_factor() - editor.get_scale()).abs() {
            editor.set_scale(target.scale_factor());
        }

        if text_font.is_changed()
            || matches!(text_font.font_size, FontSize::Rem(_)) && rem_size.is_changed()
            || matches!(
                text_font.font_size,
                FontSize::Vw(_) | FontSize::Vh(_) | FontSize::VMin(_) | FontSize::VMax(_)
            ) && target.is_changed()
        {
            editor.edit_styles().insert(StyleProperty::FontSize(
                text_font.font_size.eval(target.logical_size(), rem_size.0),
            ));
        }

        if text_font.is_changed() {
            let Ok(font_family) = resolve_font_source(&text_font.font, fonts.as_ref()) else {
                continue;
            };

            let family = font_family.into_owned();
            let style_set = editable_text.editor.edit_styles();
            style_set.insert(StyleProperty::FontFamily(family));
            style_set.insert(StyleProperty::Brush(TextBrush::new(
                0,
                text_font.font_smoothing,
            )));
        }

        if line_height.is_changed() {
            let style_set = editable_text.editor.edit_styles();
            style_set.insert(StyleProperty::LineHeight(line_height.eval()));
        }

        if text_layout.is_changed() {
            let style_set = editable_text.editor.edit_styles();
            match text_layout.linebreak {
                LineBreak::AnyCharacter => {
                    style_set.insert(StyleProperty::WordBreak(parley::WordBreak::BreakAll));
                    style_set.insert(StyleProperty::OverflowWrap(parley::OverflowWrap::Normal));
                    style_set.insert(StyleProperty::TextWrapMode(parley::TextWrapMode::Wrap));
                }
                LineBreak::WordOrCharacter => {
                    style_set.insert(StyleProperty::WordBreak(parley::WordBreak::Normal));
                    style_set.insert(StyleProperty::OverflowWrap(parley::OverflowWrap::Anywhere));
                    style_set.insert(StyleProperty::TextWrapMode(parley::TextWrapMode::Wrap));
                }
                LineBreak::NoWrap => {
                    style_set.insert(StyleProperty::WordBreak(parley::WordBreak::Normal));
                    style_set.insert(StyleProperty::OverflowWrap(parley::OverflowWrap::Normal));
                    style_set.insert(StyleProperty::TextWrapMode(parley::TextWrapMode::NoWrap));
                }
                LineBreak::WordBoundary => {
                    style_set.insert(StyleProperty::WordBreak(parley::WordBreak::Normal));
                    style_set.insert(StyleProperty::OverflowWrap(parley::OverflowWrap::Normal));
                    style_set.insert(StyleProperty::TextWrapMode(parley::TextWrapMode::Wrap));
                }
            }

            editable_text
                .editor
                .set_alignment(text_layout.justify.into());
        }
    }
}

/// Refreshes the [`EditableText`]'s layout if stale and then writes it
/// it to [`TextLayoutInfo`] for rendering and picking.
/// Adds required glyphs to the texture atlas
pub fn update_editable_text_layout(
    mut font_cx: ResMut<FontCx>,
    mut layout_cx: ResMut<LayoutCx>,
    mut scale_cx: ResMut<ScaleCx>,
    mut font_atlas_set: ResMut<FontAtlasSet>,
    mut textures: ResMut<Assets<Image>>,
    mut input_field_query: Query<(
        Entity,
        &TextFont,
        Ref<FontHinting>,
        Ref<ComputedUiRenderTargetInfo>,
        &mut EditableText,
        &mut TextLayoutInfo,
        Ref<ComputedNode>,
        &mut EditableTextGeneration,
    )>,
    rem_size: Res<RemSize>,
    input_focus: Option<Res<InputFocus>>,
    mut cursor_timer: Local<Duration>,
    time: Res<Time<Real>>,
) {
    *cursor_timer += time.delta();

    for (
        entity,
        text_font,
        hinting,
        target,
        mut editable_text,
        mut info,
        computed_node,
        mut generation,
    ) in input_field_query.iter_mut()
    {
        let cursor_width = editable_text.cursor_width;
        let cursor_blink_period = editable_text.cursor_blink_period;

        if computed_node.is_changed() {
            editable_text
                .editor
                .set_width(Some(computed_node.content_box().width()));
        }

        let mut driver = editable_text
            .editor
            .driver(&mut font_cx.0, &mut layout_cx.0);

        driver.refresh_layout();

        let compose_range = driver.editor.raw_compose().clone();

        let layout_changed = driver.editor.generation() != **generation;
        if layout_changed {
            **generation = driver.editor.generation();
        }

        if layout_changed || hinting.is_changed() {
            let layout = driver.layout();

            info.scale_factor = layout.scale();
            info.size = (
                layout.width() / layout.scale(),
                layout.height() / layout.scale(),
            )
                .into();

            info.preedit_underline_rects.clear();
            info.glyphs.clear();
            info.run_geometry.clear();

            for (line_index, line) in layout.lines().enumerate() {
                for item in line.items() {
                    match item {
                        PositionedLayoutItem::GlyphRun(glyph_run) => {
                            let brush = glyph_run.style().brush;

                            let run = glyph_run.run();

                            let font_data = run.font();
                            let font_size = run.font_size();
                            let coords = run.normalized_coords();

                            let font_atlas_key = FontAtlasKey {
                                id: font_data.data.id() as u32,
                                index: font_data.index,
                                font_size_bits: font_size.to_bits(),
                                variations_hash: FixedHasher.hash_one(coords),
                                hinting: *hinting,
                                font_smoothing: brush.font_smoothing,
                            };

                            for glyph in glyph_run.positioned_glyphs() {
                                let font_atlases =
                                    font_atlas_set.entry(font_atlas_key).or_default();
                                let Ok(atlas_info) = get_glyph_atlas_info(
                                    font_atlases,
                                    GlyphCacheKey {
                                        glyph_id: glyph.id as u16,
                                    },
                                )
                                .map(Ok)
                                .unwrap_or_else(|| {
                                    let font_ref = FontRef::from_index(
                                        font_data.data.as_ref(),
                                        font_data.index as usize,
                                    )
                                    .unwrap();
                                    let mut scaler = scale_cx
                                        .builder(font_ref)
                                        .size(font_size)
                                        .hint(matches!(*hinting, FontHinting::Enabled))
                                        .normalized_coords(coords)
                                        .build();
                                    add_glyph_to_atlas(
                                        font_atlases,
                                        textures.as_mut(),
                                        &mut scaler,
                                        text_font.font_smoothing,
                                        glyph.id as u16,
                                    )
                                }) else {
                                    continue;
                                };

                                info.glyphs.push(PositionedGlyph {
                                    position: Vec2::new(glyph.x, glyph.y)
                                        + atlas_info.rect.size() / 2.
                                        + atlas_info.offset,
                                    atlas_info,
                                    section_index: brush.section_index as usize,
                                    line_index,
                                });
                            }

                            let metrics = run.metrics();
                            let underline_y = glyph_run.baseline() - metrics.underline_offset;
                            let underline_thickness = metrics.underline_size;

                            let run_text_range = run.text_range();
                            if let Some(cr) = &compose_range
                                && run_text_range.start < cr.end
                                && run_text_range.end > cr.start
                            {
                                let mut x = glyph_run.offset();
                                let mut underline_start_x = None;
                                let mut underline_end_x = x;

                                for cluster in run.visual_clusters() {
                                    let ct = cluster.text_range();
                                    if ct.start < cr.end && ct.end > cr.start {
                                        underline_start_x.get_or_insert(x);
                                        underline_end_x = x + cluster.advance();
                                    }
                                    x += cluster.advance();
                                }

                                if let Some(start_x) = underline_start_x {
                                    info.preedit_underline_rects.push(Rect {
                                        min: Vec2::new(start_x, underline_y),
                                        max: Vec2::new(
                                            underline_end_x,
                                            underline_y + underline_thickness,
                                        ),
                                    });
                                }
                            }

                            info.run_geometry.push(RunGeometry {
                                section_index: brush.section_index as usize,
                                bounds: Rect {
                                    min: Vec2::new(glyph_run.offset(), line.metrics().min_coord),
                                    max: Vec2::new(
                                        glyph_run.offset() + glyph_run.advance(),
                                        line.metrics().max_coord,
                                    ),
                                },
                                strikethrough_y: glyph_run.baseline()
                                    - metrics.strikethrough_offset,
                                strikethrough_thickness: metrics.strikethrough_size,
                                underline_y,
                                underline_thickness,
                            });
                        }
                        PositionedLayoutItem::InlineBox(_inline) => {
                            // TODO: handle inline boxes
                        }
                    }
                }
            }

            info.selection_rects = driver
                .editor
                .selection_geometry()
                .iter()
                .map(|&b| bounding_box_to_rect(b.0))
                .collect();
        }

        if let Some(input_focus) = input_focus.as_ref()
            && Some(entity) == input_focus.get()
        {
            if input_focus.is_changed() || layout_changed || *cursor_timer >= cursor_blink_period {
                *cursor_timer = Duration::ZERO;
            }

            if *cursor_timer < cursor_blink_period / 2 {
                info.cursor = driver
                    .editor
                    .cursor_geometry(
                        cursor_width * text_font.font_size.eval(target.logical_size(), rem_size.0),
                    )
                    .map(bounding_box_to_rect);
            } else {
                info.cursor = None;
            }
        } else {
            info.cursor = None;
        }
    }
}

fn bounding_box_to_rect(geom: BoundingBox) -> Rect {
    Rect {
        min: Vec2 {
            x: geom.x0 as f32,
            y: geom.y0 as f32,
        },
        max: Vec2 {
            x: geom.x1 as f32,
            y: geom.y1 as f32,
        },
    }
}

/// Scroll editable text to keep cursor in view after edits.
pub fn scroll_editable_text(
    mut query: Query<
        (&EditableText, &mut TextScroll, &ComputedNode),
        Changed<EditableTextGeneration>,
    >,
) {
    for (editable_text, mut scroll, node) in query.iter_mut() {
        let view_size = node.content_box().size();
        if view_size.cmple(Vec2::ZERO).any() {
            continue;
        }

        let Some(cursor) = editable_text
            .editor
            .cursor_geometry(1.0)
            .map(bounding_box_to_rect)
        else {
            continue;
        };

        let mut new_scroll = scroll.0;

        if cursor.min.x < new_scroll.x {
            new_scroll.x = cursor.min.x;
        } else if new_scroll.x + view_size.x < cursor.max.x {
            new_scroll.x = cursor.max.x - view_size.x;
        }

        if cursor.min.y < new_scroll.y {
            new_scroll.y = cursor.min.y;
        } else if new_scroll.y + view_size.y < cursor.max.y {
            new_scroll.y = cursor.max.y - view_size.y;
        }

        new_scroll = new_scroll.max(Vec2::ZERO);

        if scroll.0 != new_scroll {
            scroll.0 = new_scroll;
        }
    }
}
