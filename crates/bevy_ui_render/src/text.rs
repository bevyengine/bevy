use bevy_asset::AssetId;
use bevy_camera::visibility::InheritedVisibility;
use bevy_color::Alpha;
use bevy_ecs::prelude::*;
use bevy_math::{Affine2, Rect, Vec2};
use bevy_render::{sync_world::TemporaryRenderEntity, Extract};
use bevy_sprite::BorderRect;
use bevy_text::{TextCursorStyle, TextLayoutInfo};
use bevy_ui::{
    widget::TextScroll, CalculatedClip, ComputedNode, ComputedStackIndex, ComputedUiTargetCamera,
    ResolvedBorderRadius, UiGlobalTransform,
};

use crate::{
    stack_z_offsets, ExtractedUiItem, ExtractedUiNode, ExtractedUiNodes, NodeType, UiCameraMap,
};

pub fn extract_text_cursor(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
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
            Option<&TextScroll>,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
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
        text_scroll,
    ) in text_node_query.iter()
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
                uinode.content_box().min - text_scroll.map_or(Vec2::ZERO, |s| s.0),
            );

        let clip = if text_scroll.is_some() {
            let content_box = uinode.content_box();
            let text_clip = Rect::from_center_size(
                global_transform.affine().translation + content_box.center(),
                content_box.size(),
            );
            Some(maybe_clip.map_or(text_clip, |clip| clip.clip.intersect(text_clip)))
        } else {
            maybe_clip.map(|clip| clip.clip)
        };

        if !text_layout_info.selection_rects.is_empty()
            && !cursor_style.selection_color.is_fully_transparent()
        {
            let selection_color = cursor_style.selection_color.to_linear();

            for selection in text_layout_info.selection_rects.iter() {
                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
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
                        border_radius: ResolvedBorderRadius::default(),
                        node_type: NodeType::Rect,
                    },
                    main_entity: entity.into(),
                });
            }
        }

        if let Some(cursor_rect) = text_layout_info.cursor
            && !cursor_rect.is_empty()
            && !cursor_style.color.is_fully_transparent()
        {
            extracted_uinodes.uinodes.push(ExtractedUiNode {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
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
                main_entity: entity.into(),
            });
        }
    }
}
