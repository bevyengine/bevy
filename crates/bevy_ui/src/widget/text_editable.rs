use core::hash::BuildHasher;

use crate::{ComputedNode, ComputedUiRenderTargetInfo, ContentSize, FixedMeasure, NodeMeasure};
use bevy_asset::Assets;

use bevy_ecs::{
    change_detection::DetectChanges,
    system::{Query, Res, ResMut},
    world::Ref,
};
use bevy_image::prelude::*;
use bevy_math::{Rect, Vec2};
use bevy_platform::hash::FixedHasher;
use bevy_text::*;
use bevy_text::{
    add_glyph_to_atlas, get_glyph_atlas_info, FontAtlasKey, FontAtlasSet, FontCx, GlyphCacheKey,
    LayoutCx, LineHeight, RunGeometry, ScaleCx, TextFont, TextLayoutInfo,
};
use parley::{swash::FontRef, BoundingBox};
use parley::{FontFamily, FontStack, PositionedLayoutItem};

/// Updates [`EditableText::editor`] to match e.g. [`TextFont`]
/// Writes layout to [`TextLayoutInfo`] for rendering
/// Adds required glyphs to the texture atlas
// TODO: add change detection logic here to improve performance
pub fn editable_text_system(
    fonts: Res<Assets<Font>>,
    mut font_cx: ResMut<FontCx>,
    mut layout_cx: ResMut<LayoutCx>,
    mut scale_cx: ResMut<ScaleCx>,
    mut font_atlas_set: ResMut<FontAtlasSet>,
    mut textures: ResMut<Assets<Image>>,
    mut input_field_query: Query<(
        &TextFont,
        &LineHeight,
        &FontHinting,
        Ref<ComputedUiRenderTargetInfo>,
        &mut EditableText,
        &mut TextLayoutInfo,
        &mut ContentSize,
        Ref<ComputedNode>,
    )>,
    rem_size: Res<RemSize>,
) {
    for (
        text_font,
        line_height,
        hinting,
        target,
        mut editable_text,
        mut info,
        mut content_size,
        computed_node,
    ) in input_field_query.iter_mut()
    {
        let Ok(font_family) = resolve_font_source(&text_font.font, fonts.as_ref()) else {
            continue;
        };

        let family = match font_family {
            FontFamily::Named(name) => FontFamily::Named(name.into_owned().into()),
            FontFamily::Generic(generic) => FontFamily::Generic(generic),
        };
        let style_set = editable_text.editor.edit_styles();
        style_set.insert(parley::StyleProperty::LineHeight(line_height.eval()));
        style_set.insert(parley::StyleProperty::FontStack(FontStack::Single(family)));

        let logical_viewport_size = target.logical_size();
        let font_size = text_font.font_size.eval(logical_viewport_size, rem_size.0);
        style_set.insert(parley::StyleProperty::FontSize(font_size));

        if target.is_changed() {
            editable_text.editor.set_scale(target.scale_factor());
        }

        if computed_node.is_changed() {
            editable_text.editor.set_width(Some(computed_node.size().x));
        }

        let mut driver = editable_text
            .editor
            .driver(&mut font_cx.0, &mut layout_cx.0);

        driver.refresh_layout();

        let layout = driver.layout();

        info.scale_factor = layout.scale();
        info.size = (
            layout.width() / layout.scale(),
            layout.height() / layout.scale(),
        )
            .into();

        content_size.set(NodeMeasure::Fixed(FixedMeasure {
            size: info.size * target.scale_factor(),
        }));

        info.glyphs.clear();
        info.run_geometry.clear();

        for line in layout.lines() {
            for (line_index, item) in line.items().enumerate() {
                match item {
                    PositionedLayoutItem::GlyphRun(glyph_run) => {
                        let (span_index, smoothing) = glyph_run.style().brush;

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
                            font_smoothing: smoothing,
                        };

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
                                let font_ref = FontRef::from_index(
                                    font_data.data.as_ref(),
                                    font_data.index as usize,
                                )
                                .unwrap();
                                let mut scaler = scale_cx
                                    .builder(font_ref)
                                    .size(font_size)
                                    .hint(true)
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
                            strikethrough_y: glyph_run.baseline()
                                - run.metrics().strikethrough_offset,
                            strikethrough_thickness: run.metrics().strikethrough_size,
                            underline_y: glyph_run.baseline() - run.metrics().underline_offset,
                            underline_thickness: run.metrics().underline_size,
                        });
                    }
                    PositionedLayoutItem::InlineBox(_inline) => {
                        // TODO: handle inline boxes
                    }
                }
            }
        }

        let geom = editable_text
            .editor
            .cursor_geometry(editable_text.cursor_width * font_size);

        info.cursor = geom.map(bounding_box_to_rect);

        info.selection_rects = editable_text
            .editor
            .selection_geometry()
            .iter()
            .map(|&b| bounding_box_to_rect(b.0))
            .collect();
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
