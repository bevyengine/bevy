use crate::{
    clipping::clip_polygon,
    extract_layout::{ExtractedUiLayout, ExtractedUiNodeLayout},
    shader_flags, stack_z_offsets, DrawUi, TransparentUi, UiAntiAlias, UiCameraView, UiMeta,
    UiPipeline, UiPipelineKey, UiVertex, QUAD_VERTEX_POSITIONS,
};
use bevy_asset::AssetId;
use bevy_color::{Alpha, ColorToComponents, LinearRgba};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_input_focus::InputFocus;
use bevy_math::{Affine2, FloatOrd, Rect, Vec2};
use bevy_platform::collections::hash_map::Entry;
use bevy_render::{
    render_asset::RenderAssets,
    render_phase::{DrawFunctions, PhaseItemExtraIndex, ViewSortedRenderPhases},
    render_resource::{PipelineCache, SpecializedRenderPipelines},
    sync_world::{MainEntity, MainEntityHashMap, MainEntityHashSet},
    texture::GpuImage,
    view::ExtractedView,
    Extract,
};
use bevy_text::{
    ComputedTextBlock, EditableText, PositionedGlyph, Strikethrough, StrikethroughColor,
    TextBackgroundColor, TextColor, TextCursorStyle, TextLayoutInfo, TextReadWriteMode, TextSpan,
    Underline, UnderlineColor,
};
use bevy_ui::{
    widget::{Text, TextShadow},
    CalculatedClip, ResolvedBorderRadius, UiGlobalTransform,
};
use core::ops::Range;

pub struct ExtractedGlyph {
    pub color: LinearRgba,
    pub translation: Vec2,
    pub rect: Rect,
}

pub struct Sections {
    pub range: Range<u32>,
    pub atlas_texture: AssetId<Image>,
}

pub struct ExtractedGlyphLayout {
    pub shadow_color: LinearRgba,
    pub shadow_offset: Vec2,
    pub cursor_style: TextCursorStyle,
    pub glyphs: Vec<ExtractedGlyph>,
    pub strike_through: Vec<(Rect, LinearRgba)>,
    pub underline: Vec<(Rect, LinearRgba)>,
    pub sections: Vec<Sections>,
    pub backgrounds: Vec<(Rect, LinearRgba)>,
    pub selections: Vec<(Rect, ResolvedBorderRadius)>,
    pub cursor: Option<Rect>,
    pub focused: bool,
    pub viewport_offset: Vec2,
    pub clip_to_content_box: bool,
}

#[derive(Resource, Default)]
pub struct ExtractedGlyphLayouts {
    pub uinodes: MainEntityHashMap<(Entity, ExtractedGlyphLayout)>,
    pub changed: MainEntityHashSet,
}

pub fn extract_text(
    mut commands: Commands,
    mut extracted_glyph_layouts: ResMut<ExtractedGlyphLayouts>,
    changed_text_query: Extract<
        Query<
            Entity,
            (
                Or<(With<Text>, With<EditableText>)>,
                Or<(
                    Or<(
                        Changed<ComputedTextBlock>,
                        Changed<TextColor>,
                        Changed<TextLayoutInfo>,
                        Changed<TextCursorStyle>,
                        Changed<TextShadow>,
                        Changed<EditableText>,
                    )>,
                    Or<(
                        Changed<TextReadWriteMode>,
                        Changed<TextBackgroundColor>,
                        Changed<Strikethrough>,
                        Changed<Underline>,
                        Changed<StrikethroughColor>,
                        Changed<UnderlineColor>,
                    )>,
                )>,
            ),
        >,
    >,
    changed_text_span_query: Extract<
        Query<
            Entity,
            (
                With<TextSpan>,
                Or<(
                    Changed<TextColor>,
                    Changed<TextBackgroundColor>,
                    Changed<Strikethrough>,
                    Changed<Underline>,
                    Changed<StrikethroughColor>,
                    Changed<UnderlineColor>,
                )>,
            ),
        >,
    >,
    text_query: Extract<
        Query<(
            Entity,
            &ComputedTextBlock,
            &TextColor,
            &TextLayoutInfo,
            Option<&TextReadWriteMode>,
            Option<&EditableText>,
            Option<&TextCursorStyle>,
            Option<&TextShadow>,
        )>,
    >,
    text_styles: Extract<
        Query<(
            AnyOf<(&TextBackgroundColor, &Strikethrough, &Underline)>,
            &TextColor,
            Option<&StrikethroughColor>,
            Option<&UnderlineColor>,
        )>,
    >,
    text_span_parent_query: Extract<Query<&ChildOf, With<TextSpan>>>,
    text_root_query: Extract<Query<Entity, With<Text>>>,
    (
        mut removed_text_query,
        mut removed_computed_text_block_query,
        mut removed_text_color_query,
        mut removed_text_layout_info_query,
        mut removed_text_cursor_style_query,
        mut removed_text_shadow_query,
        mut removed_editable_text_query,
    ): (
        Extract<RemovedComponents<Text>>,
        Extract<RemovedComponents<ComputedTextBlock>>,
        Extract<RemovedComponents<TextColor>>,
        Extract<RemovedComponents<TextLayoutInfo>>,
        Extract<RemovedComponents<TextCursorStyle>>,
        Extract<RemovedComponents<TextShadow>>,
        Extract<RemovedComponents<EditableText>>,
    ),
    (
        mut removed_text_background_color_query,
        mut removed_strikethrough_query,
        mut removed_underline_query,
        mut removed_strikethrough_color_query,
        mut removed_underline_color_query,
        mut removed_read_write_mode_query,
    ): (
        Extract<RemovedComponents<TextBackgroundColor>>,
        Extract<RemovedComponents<Strikethrough>>,
        Extract<RemovedComponents<Underline>>,
        Extract<RemovedComponents<StrikethroughColor>>,
        Extract<RemovedComponents<UnderlineColor>>,
        Extract<RemovedComponents<TextReadWriteMode>>,
    ),
    input_focus: Extract<Option<Res<InputFocus>>>,
    mut nodes_to_extract: Local<MainEntityHashSet>,
) {
    extracted_glyph_layouts.changed.clear();
    nodes_to_extract.clear();
    nodes_to_extract.extend(changed_text_query.iter().map(MainEntity::from));
    if input_focus.as_ref().is_some_and(DetectChanges::is_changed) {
        nodes_to_extract.extend(text_root_query.iter().map(MainEntity::from));
    }
    nodes_to_extract.extend(
        removed_text_query
            .read()
            .chain(removed_computed_text_block_query.read())
            .chain(removed_text_layout_info_query.read())
            .chain(removed_text_cursor_style_query.read())
            .chain(removed_text_shadow_query.read())
            .chain(removed_editable_text_query.read())
            .chain(removed_read_write_mode_query.read())
            .map(MainEntity::from),
    );

    for entity in changed_text_span_query
        .iter()
        .chain(removed_text_color_query.read())
        .chain(removed_text_background_color_query.read())
        .chain(removed_strikethrough_query.read())
        .chain(removed_underline_query.read())
        .chain(removed_strikethrough_color_query.read())
        .chain(removed_underline_color_query.read())
    {
        let mut entity = entity;
        loop {
            if text_root_query.contains(entity) {
                nodes_to_extract.insert(entity.into());
                break;
            }
            match text_span_parent_query.get(entity) {
                Ok(parent) => entity = parent.parent(),
                Err(_) => break,
            }
        }
    }

    for main_entity in nodes_to_extract.drain() {
        extracted_glyph_layouts.changed.insert(main_entity);

        let Ok((
            entity,
            computed_block,
            text_color,
            text_layout_info,
            rwmode,
            editable_text,
            cursor_style,
            shadow,
        )) = text_query.get(main_entity.entity())
        else {
            let Some((render_entity, _)) = extracted_glyph_layouts.uinodes.remove(&main_entity)
            else {
                continue;
            };
            commands.entity(render_entity).despawn();
            continue;
        };

        let has_cursor_style = cursor_style.is_some();
        let cursor_style = cursor_style.copied().unwrap_or_default();
        let (shadow_color, shadow_offset) = shadow
            .filter(|shadow| !shadow.color.is_fully_transparent())
            .map_or((LinearRgba::NONE, Vec2::ZERO), |shadow| {
                (shadow.color.into(), shadow.offset)
            });
        let viewport_offset =
            editable_text.map_or(Vec2::ZERO, |editable_text| editable_text.viewport.offset);
        let clip_to_content_box = editable_text.is_some();
        let focused = input_focus
            .as_ref()
            .is_some_and(|input_focus| input_focus.get() == Some(entity));

        let selection_color =
            if focused && rwmode.is_some_and(|rwmode| *rwmode == TextReadWriteMode::Editable) {
                cursor_style.selection_color
            } else {
                cursor_style.unfocused_selection_color
            };
        let mut selections = vec![];
        if has_cursor_style
            && !text_layout_info.selection_rects.is_empty()
            && !selection_color.is_fully_transparent()
            && rwmode.is_some_and(|rwmode| *rwmode != TextReadWriteMode::Static)
        {
            let selection_radius = cursor_style.selection_radius.clamp(0.0, 0.5);

            for (prev, selection, next) in
                text_layout_info
                    .selection_rects
                    .iter()
                    .enumerate()
                    .map(|(i, current)| {
                        (
                            i.checked_sub(1)
                                .map(|i| text_layout_info.selection_rects[i]),
                            *current,
                            text_layout_info.selection_rects.get(i + 1).copied(),
                        )
                    })
            {
                let radius = selection.height() * selection_radius;
                let mut border_radius = ResolvedBorderRadius {
                    top_left: Vec2::splat(radius),
                    top_right: Vec2::splat(radius),
                    bottom_right: Vec2::splat(radius),
                    bottom_left: Vec2::splat(radius),
                };

                if let Some(prev) = prev {
                    if selection.min.x <= prev.max.x {
                        border_radius.top_left.x = (prev.min.x - selection.min.x).clamp(0., radius);
                    }
                    if prev.min.x <= selection.max.x {
                        border_radius.top_right.x =
                            (selection.max.x - prev.max.x).clamp(0., radius);
                    }
                }

                if let Some(next) = next {
                    if selection.min.x <= next.max.x {
                        border_radius.bottom_left.x =
                            (next.min.x - selection.min.x).clamp(0., radius);
                    }
                    if next.min.x <= selection.max.x {
                        border_radius.bottom_right.x =
                            (selection.max.x - next.max.x).clamp(0., radius);
                    }
                }

                selections.push((selection, border_radius));
            }
        }

        let mut backgrounds = vec![];
        let mut strike_through = vec![];
        let mut underline = vec![];

        for run in text_layout_info.run_geometry.iter() {
            let Some(section_entity) = computed_block
                .entities()
                .get(run.section_index as usize)
                .map(|text_entity| text_entity.entity)
            else {
                continue;
            };
            let Ok((
                (text_background_color, maybe_strikethrough, maybe_underline),
                run_text_color,
                maybe_strikethrough_color,
                maybe_underline_color,
            )) = text_styles.get(section_entity)
            else {
                continue;
            };

            if let Some(text_background_color) = text_background_color {
                backgrounds.push((run.bounds, text_background_color.0.to_linear()));
            }

            if maybe_strikethrough.is_some() {
                let rect =
                    Rect::from_center_size(run.strikethrough_position(), run.strikethrough_size());
                strike_through.push((
                    rect,
                    maybe_strikethrough_color
                        .map(|color| color.0)
                        .unwrap_or(run_text_color.0)
                        .to_linear(),
                ));
            }

            if maybe_underline.is_some() {
                let rect = Rect::from_center_size(run.underline_position(), run.underline_size());
                underline.push((
                    rect,
                    maybe_underline_color
                        .map(|color| color.0)
                        .unwrap_or(run_text_color.0)
                        .to_linear(),
                ));
            }
        }

        let mut color = text_color.0.to_linear();
        let selected_text_color = cursor_style
            .selected_text_color
            .map(|selected_text_color| selected_text_color.to_linear());
        let mut current_section_index = 0;
        let mut section_start = 0;
        let mut glyphs = vec![];
        let mut sections = vec![];

        for (
            i,
            PositionedGlyph {
                position,
                atlas_info,
                section_index,
                ..
            },
        ) in text_layout_info.glyphs.iter().enumerate()
        {
            if current_section_index != *section_index
                && let Some(section_entity) = computed_block
                    .entities()
                    .get(*section_index as usize)
                    .map(|text_entity| text_entity.entity)
            {
                color = text_styles
                    .get(section_entity)
                    .map(|(_, text_color, _, _)| LinearRgba::from(text_color.0))
                    .unwrap_or_default();
                current_section_index = *section_index;
            }

            let color = if !atlas_info.is_alpha_mask {
                LinearRgba::WHITE
            } else if let Some(selected_text_color) = selected_text_color
                && text_layout_info
                    .selection_rects
                    .iter()
                    .any(|selection_rect| {
                        let glyph_rect = Rect::from_center_size(*position, atlas_info.rect.size());
                        selection_rect.contains(glyph_rect.min)
                            && selection_rect.contains(glyph_rect.max)
                    })
            {
                selected_text_color
            } else {
                color
            };

            glyphs.push(ExtractedGlyph {
                color,
                translation: *position,
                rect: atlas_info.rect,
            });

            if text_layout_info
                .glyphs
                .get(i + 1)
                .is_none_or(|info| info.atlas_info.texture != atlas_info.texture)
            {
                let glyph_count = glyphs.len() as u32;
                sections.push(Sections {
                    range: section_start..glyph_count,
                    atlas_texture: atlas_info.texture,
                });
                section_start = glyph_count;
            }
        }

        underline.extend(
            text_layout_info
                .preedit_underline_rects
                .iter()
                .map(|rect| (*rect, text_color.0.to_linear())),
        );

        let cursor = if has_cursor_style
            && let Some((true, cursor)) = text_layout_info.cursor
            && !cursor.is_empty()
            && !cursor_style.color.is_fully_transparent()
            && rwmode.is_some_and(|rwmode| *rwmode != TextReadWriteMode::Static)
        {
            Some(cursor)
        } else {
            None
        };

        if glyphs.is_empty()
            && strike_through.is_empty()
            && underline.is_empty()
            && backgrounds.is_empty()
            && selections.is_empty()
            && cursor.is_none()
        {
            if let Some((render_entity, _)) = extracted_glyph_layouts.uinodes.remove(&main_entity) {
                commands.entity(render_entity).despawn();
            }
            continue;
        }

        let layout = ExtractedGlyphLayout {
            shadow_color,
            shadow_offset,
            cursor_style,
            glyphs,
            strike_through,
            underline,
            sections,
            backgrounds,
            selections,
            cursor,
            focused,
            viewport_offset,
            clip_to_content_box,
        };

        match extracted_glyph_layouts.uinodes.entry(entity.into()) {
            Entry::Occupied(mut entry) => entry.get_mut().1 = layout,
            Entry::Vacant(entry) => {
                entry.insert((commands.spawn_empty().id(), layout));
            }
        }
    }
}

pub fn queue_text(
    extracted_glyph_layouts: Res<ExtractedGlyphLayouts>,
    extracted_layout: Res<ExtractedUiLayout>,
    ui_pipeline: Res<UiPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiPipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    render_views: Query<(&UiCameraView, Option<&UiAntiAlias>), With<ExtractedView>>,
    camera_views: Query<&ExtractedView>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawUi>();
    let mut current_camera_entity = Entity::PLACEHOLDER;
    let mut current_phase = None;

    for (main_entity, (render_entity, _)) in extracted_glyph_layouts.uinodes.iter() {
        let Some(layout) = extracted_layout.layout.get(main_entity) else {
            continue;
        };

        if !layout.visible {
            continue;
        }

        if current_camera_entity != layout.extracted_camera {
            current_phase = render_views.get(layout.extracted_camera).ok().and_then(
                |(default_camera_view, ui_anti_alias)| {
                    camera_views
                        .get(default_camera_view.0)
                        .ok()
                        .and_then(|view| {
                            transparent_render_phases
                                .get_mut(&view.retained_view_entity)
                                .map(|transparent_phase| {
                                    let pipeline = pipelines.specialize(
                                        &pipeline_cache,
                                        &ui_pipeline,
                                        UiPipelineKey {
                                            target_format: view.target_format,
                                            anti_alias: matches!(
                                                ui_anti_alias,
                                                None | Some(UiAntiAlias::On)
                                            ),
                                        },
                                    );
                                    (pipeline, transparent_phase)
                                })
                        })
                },
            );
            current_camera_entity = layout.extracted_camera;
        }

        let Some((pipeline, transparent_phase)) = current_phase.as_mut() else {
            continue;
        };

        transparent_phase.add_transient(TransparentUi {
            draw_function,
            pipeline: *pipeline,
            entity: (*render_entity, *main_entity),
            sort_key: FloatOrd(layout.stack_index as f32 + stack_z_offsets::TEXT),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            indexed: true,
        });
    }
}

pub(crate) fn push_text_vertices(
    ui_meta: &mut UiMeta,
    node_layout: &ExtractedUiNodeLayout,
    glyph_layout: &ExtractedGlyphLayout,
    gpu_images: &RenderAssets<GpuImage>,
    initial_texture: AssetId<Image>,
    texture_changes_start: usize,
) {
    let shadow_offset = glyph_layout.shadow_offset / node_layout.uinode.inverse_scale_factor();
    let transform = node_layout.transform
        * Affine2::from_translation(
            node_layout.uinode.content_box().min - glyph_layout.viewport_offset,
        );
    let text_clip;
    let clip = if glyph_layout.clip_to_content_box {
        text_clip = node_layout.clip.clone().unwrap_or_default().with_rect(
            node_layout.uinode.content_box(),
            &UiGlobalTransform::from(node_layout.transform),
        );
        Some(&text_clip)
    } else {
        node_layout.clip.as_ref()
    };
    let (mut texture_range_start, mut current_texture) =
        if texture_changes_start < ui_meta.texture_changes.len() {
            ui_meta
                .texture_changes
                .pop()
                .map(|(range, texture)| (range.start, texture))
                .unwrap()
        } else {
            (ui_meta.indices.len() as u32, initial_texture)
        };

    let selection_color = if glyph_layout.focused {
        glyph_layout.cursor_style.selection_color
    } else {
        glyph_layout.cursor_style.unfocused_selection_color
    }
    .to_linear();
    for (rect, radius) in &glyph_layout.selections {
        push_untextured_vertices(ui_meta, transform, clip, *rect, selection_color, *radius);
    }
    for (rect, color) in &glyph_layout.backgrounds {
        push_untextured_vertices(
            ui_meta,
            transform,
            clip,
            *rect,
            *color,
            ResolvedBorderRadius::ZERO,
        );
    }

    if !glyph_layout.shadow_color.is_fully_transparent() {
        for section in &glyph_layout.sections {
            let Some(gpu_image) = gpu_images.get(section.atlas_texture) else {
                continue;
            };
            let range_end = ui_meta.indices.len() as u32;
            if current_texture != section.atlas_texture {
                if texture_range_start < range_end {
                    ui_meta
                        .texture_changes
                        .push((texture_range_start..range_end, current_texture));
                }
                texture_range_start = range_end;
                current_texture = section.atlas_texture;
            }
            let atlas_extent = gpu_image.size_2d().as_vec2();
            for glyph in
                &glyph_layout.glyphs[section.range.start as usize..section.range.end as usize]
            {
                push_glyph_vertices(
                    ui_meta,
                    transform,
                    clip,
                    Rect::from_center_size(glyph.translation + shadow_offset, glyph.rect.size()),
                    glyph_layout.shadow_color,
                    glyph.rect,
                    atlas_extent,
                );
            }
        }
        for (rect, _) in glyph_layout
            .strike_through
            .iter()
            .chain(glyph_layout.underline.iter())
        {
            push_untextured_vertices(
                ui_meta,
                transform,
                clip,
                Rect::from_center_size(rect.center() + shadow_offset, rect.size()),
                glyph_layout.shadow_color,
                ResolvedBorderRadius::ZERO,
            );
        }
    }

    for section in &glyph_layout.sections {
        let Some(gpu_image) = gpu_images.get(section.atlas_texture) else {
            continue;
        };
        let range_end = ui_meta.indices.len() as u32;
        if current_texture != section.atlas_texture {
            if texture_range_start < range_end {
                ui_meta
                    .texture_changes
                    .push((texture_range_start..range_end, current_texture));
            }
            texture_range_start = range_end;
            current_texture = section.atlas_texture;
        }
        let atlas_extent = gpu_image.size_2d().as_vec2();
        for glyph in &glyph_layout.glyphs[section.range.start as usize..section.range.end as usize]
        {
            push_glyph_vertices(
                ui_meta,
                transform,
                clip,
                Rect::from_center_size(glyph.translation, glyph.rect.size()),
                glyph.color,
                glyph.rect,
                atlas_extent,
            );
        }
    }
    for (rect, color) in glyph_layout
        .strike_through
        .iter()
        .chain(glyph_layout.underline.iter())
    {
        push_untextured_vertices(
            ui_meta,
            transform,
            clip,
            *rect,
            *color,
            ResolvedBorderRadius::ZERO,
        );
    }
    if let Some(cursor) = glyph_layout.cursor {
        push_untextured_vertices(
            ui_meta,
            transform,
            clip,
            cursor,
            glyph_layout.cursor_style.color.to_linear(),
            ResolvedBorderRadius::ZERO,
        );
    }

    let range_end = ui_meta.indices.len() as u32;
    if texture_range_start < range_end {
        ui_meta
            .texture_changes
            .push((texture_range_start..range_end, current_texture));
    }
}

fn push_untextured_vertices(
    ui_meta: &mut UiMeta,
    transform: Affine2,
    clip: Option<&CalculatedClip>,
    rect: Rect,
    color: LinearRgba,
    border_radius: ResolvedBorderRadius,
) {
    let rect_size = rect.size();
    let rect_transform = transform * Affine2::from_translation(rect.center());
    let points = QUAD_VERTEX_POSITIONS.map(|pos| pos * rect_size);
    let positions = points.map(|pos| rect_transform.transform_point2(pos));
    let uvs = [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y];
    let vertices = clip_polygon(
        clip,
        &[
            (positions[0], (uvs[0], points[0])),
            (positions[1], (uvs[1], points[1])),
            (positions[2], (uvs[2], points[2])),
            (positions[3], (uvs[3], points[3])),
        ],
        |a, b, t| (a.0.lerp(b.0, t), a.1.lerp(b.1, t)),
    );
    if vertices.is_empty() {
        return;
    }

    let vertex_start = ui_meta.vertices.len() as u32;
    let color = color.to_f32_array();
    for &(position, (uv, point)) in &vertices {
        ui_meta.vertices.push(UiVertex {
            position: position.extend(0.).into(),
            uv: uv.into(),
            color,
            flags: shader_flags::UNTEXTURED,
            radius: border_radius.into(),
            border: [0.; 4],
            size: rect_size.into(),
            point: point.into(),
        });
    }
    for i in 1..vertices.len() as u32 - 1 {
        ui_meta.indices.push(vertex_start);
        ui_meta.indices.push(vertex_start + i);
        ui_meta.indices.push(vertex_start + i + 1);
    }
}

fn push_glyph_vertices(
    ui_meta: &mut UiMeta,
    transform: Affine2,
    clip: Option<&CalculatedClip>,
    rect: Rect,
    color: LinearRgba,
    glyph_rect: Rect,
    atlas_extent: Vec2,
) {
    let rect_size = rect.size();
    let rect_transform = transform * Affine2::from_translation(rect.center());
    let points = QUAD_VERTEX_POSITIONS.map(|pos| pos * rect_size);
    let positions = points.map(|pos| rect_transform.transform_point2(pos));
    let uvs = [
        Vec2::new(glyph_rect.min.x, glyph_rect.min.y),
        Vec2::new(glyph_rect.max.x, glyph_rect.min.y),
        Vec2::new(glyph_rect.max.x, glyph_rect.max.y),
        Vec2::new(glyph_rect.min.x, glyph_rect.max.y),
    ]
    .map(|pos| pos / atlas_extent);
    let vertices = clip_polygon(
        clip,
        &[
            (positions[0], (uvs[0], points[0])),
            (positions[1], (uvs[1], points[1])),
            (positions[2], (uvs[2], points[2])),
            (positions[3], (uvs[3], points[3])),
        ],
        |a, b, t| (a.0.lerp(b.0, t), a.1.lerp(b.1, t)),
    );
    if vertices.is_empty() {
        return;
    }

    let vertex_start = ui_meta.vertices.len() as u32;
    let color = color.to_f32_array();
    for &(position, (uv, point)) in &vertices {
        ui_meta.vertices.push(UiVertex {
            position: position.extend(0.).into(),
            uv: uv.into(),
            color,
            flags: shader_flags::TEXTURED,
            radius: ResolvedBorderRadius::ZERO.into(),
            border: [0.; 4],
            size: rect_size.into(),
            point: point.into(),
        });
    }
    for i in 1..vertices.len() as u32 - 1 {
        ui_meta.indices.push(vertex_start);
        ui_meta.indices.push(vertex_start + i);
        ui_meta.indices.push(vertex_start + i + 1);
    }
}
