use bevy_asset::AssetId;
use bevy_camera::visibility::InheritedVisibility;
use bevy_color::Alpha;
use bevy_ecs::prelude::*;
use bevy_input_focus::InputFocus;
use bevy_math::{Affine2, Rect, Vec2};
use bevy_render::Extract;
use bevy_sprite::BorderRect;
use bevy_text::{EditableText, TextColor, TextCursorStyle, TextLayoutInfo};
use bevy_ui::{
    CalculatedClip, ComputedNode, ComputedStackIndex, ComputedUiTargetCamera, ResolvedBorderRadius,
    UiGlobalTransform,
};

use crate::{
    stack_z_offsets, ExtractedUiItem, ExtractedUiNode, ExtractedUiNodes, NodeType, UiCameraMap,
};

pub fn extract_text_cursor(
    mut commands: Commands,
    extracted_uinodes: ResMut<ExtractedUiNodes>,
    text_node_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedStackIndex,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &TextLayoutInfo,
            &TextCursorStyle,
            Option<&EditableText>,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
    input_focus: Extract<Option<Res<InputFocus>>>,
) {
    let extracted_uinodes = extracted_uinodes.into_inner();
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        uinode,
        stack_index,
        global_transform,
        inherited_visibility,
        maybe_clip,
        target_camera,
        text_layout_info,
        cursor_style,
        editable_text,
    ) in extracted_uinodes
        .changed
        .iter()
        .flat_map(|main_entity| text_node_query.get(main_entity.entity()).ok())
    {
        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(target_camera) else {
            continue;
        };

        let transform = Affine2::from(global_transform)
            * Affine2::from_translation(
                uinode.content_box().min
                    - editable_text.map_or(Vec2::ZERO, |editor| editor.viewport.offset),
            );

        let clip = if editable_text.is_some() {
            let content_box = uinode.content_box();
            let text_clip = Rect::from_center_size(
                global_transform.affine().translation + content_box.center(),
                content_box.size(),
            );
            Some(maybe_clip.map_or(text_clip, |clip| clip.clip.intersect(text_clip)))
        } else {
            maybe_clip.map(|clip| clip.clip)
        };

        let mut focused = false;

        if let Some(input_focus) = input_focus.as_ref()
            && Some(entity) == input_focus.get()
        {
            focused = true;
        }

        let sc = if focused {
            cursor_style.selection_color
        } else {
            cursor_style.unfocused_selection_color
        };

        if !text_layout_info.selection_rects.is_empty() && !sc.is_fully_transparent() {
            let selection_color = sc.to_linear();
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
                            current,
                            text_layout_info.selection_rects.get(i + 1),
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

                extracted_uinodes
                    .uinodes
                    .entry(entity.into())
                    .or_default()
                    .insert(
                        commands.spawn_empty().id(),
                        ExtractedUiNode {
                            z_order: stack_index.0 as f32 + stack_z_offsets::TEXT_SELECTION,
                            clip,
                            image: AssetId::default(),
                            extracted_camera_entity,
                            transform: transform * Affine2::from_translation(selection.center()),
                            item: ExtractedUiItem::Node {
                                color: selection_color,
                                rect: Rect {
                                    min: Vec2::ZERO,
                                    max: selection.size(),
                                },
                                atlas_scaling: None,
                                flip_x: false,
                                flip_y: false,
                                border: BorderRect::default(),
                                border_radius,
                                node_type: NodeType::Rect,
                            },
                        },
                    );
            }
        }

        if let Some((true, cursor_rect)) = text_layout_info.cursor
            && !cursor_rect.is_empty()
            && !cursor_style.color.is_fully_transparent()
        {
            extracted_uinodes
                .uinodes
                .entry(entity.into())
                .or_default()
                .insert(
                    commands.spawn_empty().id(),
                    ExtractedUiNode {
                        z_order: stack_index.0 as f32 + stack_z_offsets::TEXT_CURSOR,
                        clip,
                        image: AssetId::default(),
                        extracted_camera_entity,
                        transform: transform * Affine2::from_translation(cursor_rect.center()),
                        item: ExtractedUiItem::Node {
                            color: cursor_style.color.to_linear(),
                            rect: Rect {
                                min: Vec2::ZERO,
                                max: cursor_rect.size(),
                            },
                            atlas_scaling: None,
                            flip_x: false,
                            flip_y: false,
                            border: BorderRect::default(),
                            border_radius: ResolvedBorderRadius::default(),
                            node_type: NodeType::Rect,
                        },
                    },
                );
        }
    }
}

pub fn extract_preedit_underlines(
    mut commands: Commands,
    extracted_uinodes: ResMut<ExtractedUiNodes>,
    text_node_query: Extract<
        Query<
            (
                Entity,
                &ComputedNode,
                &TextColor,
                &TextLayoutInfo,
                &UiGlobalTransform,
                &InheritedVisibility,
                Option<&CalculatedClip>,
                &ComputedUiTargetCamera,
                &ComputedStackIndex,
                &EditableText,
            ),
            With<EditableText>,
        >,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let extracted_uinodes = extracted_uinodes.into_inner();
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        uinode,
        text_color,
        text_layout_info,
        global_transform,
        inherited_visibility,
        maybe_clip,
        target_camera,
        stack_index,
        editable_text,
    ) in extracted_uinodes
        .changed
        .iter()
        .flat_map(|main_entity| text_node_query.get(main_entity.entity()).ok())
    {
        if !inherited_visibility.get()
            || uinode.is_empty()
            || text_layout_info.preedit_underline_rects.is_empty()
        {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(target_camera) else {
            continue;
        };

        let transform = Affine2::from(global_transform)
            * Affine2::from_translation(uinode.content_box().min - editable_text.viewport.offset);

        let text_clip = Rect::from_center_size(
            global_transform.affine().translation + uinode.content_box().center(),
            uinode.content_box().size(),
        );
        let clip = Some(maybe_clip.map_or(text_clip, |clip| clip.clip.intersect(text_clip)));

        let color = text_color.0.to_linear();

        for rect in text_layout_info.preedit_underline_rects.iter() {
            extracted_uinodes
                .uinodes
                .entry(entity.into())
                .or_default()
                .insert(
                    commands.spawn_empty().id(),
                    ExtractedUiNode {
                        z_order: stack_index.0 as f32 + stack_z_offsets::TEXT_STRIKETHROUGH,
                        clip,
                        image: AssetId::default(),
                        extracted_camera_entity,
                        transform: transform * Affine2::from_translation(rect.center()),
                        item: ExtractedUiItem::Node {
                            color,
                            rect: Rect {
                                min: Vec2::ZERO,
                                max: rect.size(),
                            },
                            atlas_scaling: None,
                            flip_x: false,
                            flip_y: false,
                            border: BorderRect::default(),
                            border_radius: ResolvedBorderRadius::default(),
                            node_type: NodeType::Rect,
                        },
                    },
                );
        }
    }
}
