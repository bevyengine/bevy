use crate::{
    extract_layout::ExtractedUiLayout, stack_z_offsets, DrawUi, TransparentUi, UiAntiAlias,
    UiCameraView, UiPipeline, UiPipelineKey,
};
use bevy_asset::AssetId;
use bevy_color::{Alpha, LinearRgba};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_input_focus::InputFocus;
use bevy_math::{FloatOrd, Rect, Vec2};
use bevy_platform::collections::hash_map::Entry;
use bevy_render::{
    render_phase::{DrawFunctions, PhaseItemExtraIndex, ViewSortedRenderPhases},
    render_resource::{PipelineCache, SpecializedRenderPipelines},
    sync_world::{MainEntity, MainEntityHashMap, MainEntityHashSet},
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
    ResolvedBorderRadius,
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
                With<Text>,
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
    extracted_geometry: Res<ExtractedUiLayout>,
    ui_pipeline: Res<UiPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiPipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    render_views: Query<(&UiCameraView, Option<&UiAntiAlias>), With<ExtractedView>>,
    camera_views: Query<&ExtractedView>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawUi>();

    for (main_entity, (render_entity, _)) in extracted_glyph_layouts.uinodes.iter() {
        let Some(geometry) = extracted_geometry.layout.get(main_entity) else {
            continue;
        };
        let Some((default_camera_view, ui_anti_alias)) =
            render_views.get(geometry.extracted_camera).ok()
        else {
            continue;
        };
        let Some(view) = camera_views.get(default_camera_view.0).ok() else {
            continue;
        };
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };
        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &ui_pipeline,
            UiPipelineKey {
                target_format: view.target_format,
                anti_alias: matches!(ui_anti_alias, None | Some(UiAntiAlias::On)),
            },
        );

        transparent_phase.add_transient(TransparentUi {
            draw_function,
            pipeline,
            entity: (*render_entity, *main_entity),
            sort_key: FloatOrd(geometry.stack_index as f32 + stack_z_offsets::TEXT),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            indexed: true,
        });
    }
}
