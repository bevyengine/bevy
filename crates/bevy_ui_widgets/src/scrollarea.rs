use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    hierarchy::ChildOf,
    observer::On,
    query::With,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_input::mouse::MouseScrollPixelsPerLine;
use bevy_math::{Affine2, Vec2};
use bevy_picking::events::{Pointer, Scroll};
use bevy_reflect::Reflect;
use bevy_ui::{ComputedNode, Node, OverflowAxis, ScrollPosition, UiGlobalTransform};

use crate::ScrollIntoView;

/// Marker component to enable trackpad / mouse wheel scrolling. This should be placed on an
/// entity that has overflow: scroll.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[require(ScrollPosition)]
#[reflect(Component)]
pub struct ScrollArea;

fn scrollarea_on_scroll(
    mut scroll: On<Pointer<Scroll>>,
    mut q_scroll_area: Query<(&Node, &ComputedNode, &mut ScrollPosition), With<ScrollArea>>,
    scroll_conversion_ratio: Res<MouseScrollPixelsPerLine>,
) {
    if let Ok((node, computed_node, mut scroll_pos)) = q_scroll_area.get_mut(scroll.entity) {
        scroll.propagate(false);
        let visible_size = computed_node.size() * computed_node.inverse_scale_factor;
        let content_size = computed_node.content_size() * computed_node.inverse_scale_factor;

        let can_scroll_x = node.overflow.x == OverflowAxis::Scroll;
        let can_scroll_y = node.overflow.y == OverflowAxis::Scroll;

        let scroll_delta = scroll.to_pixels(&scroll_conversion_ratio);
        let scroll_delta = Vec2::new(scroll_delta.x, scroll_delta.y);

        let max_range = (content_size - visible_size).max(Vec2::ZERO);

        if can_scroll_x {
            scroll_pos.x = (scroll_pos.x - scroll_delta.x).clamp(0.0, max_range.x);
        }

        if can_scroll_y {
            scroll_pos.y = (scroll_pos.y - scroll_delta.y).clamp(0.0, max_range.y);
        }
    }
}

fn on_scroll_into_view(
    mut scroll: On<ScrollIntoView>,
    q_node: Query<(&Node, &UiGlobalTransform, &ComputedNode)>,
    q_parents: Query<&ChildOf>,
    mut q_scroll_area: Query<&mut ScrollPosition, With<ScrollArea>>,
) {
    if let Ok((_target_node, target_transform, target_computed_node)) = q_node.get(scroll.entity) {
        scroll.propagate(false);
        let target_affine: Affine2 = target_transform.into();
        let target_size = target_computed_node.size() * target_computed_node.inverse_scale_factor;
        let target_pos = target_affine.translation * target_computed_node.inverse_scale_factor
            - target_size * 0.5;

        let Some(scroll_area_id) = q_parents
            .iter_ancestors(scroll.entity)
            .find(|id| q_scroll_area.contains(*id))
        else {
            return;
        };

        let (scroll_area_node, scroll_area_transform, scroll_area_computed_node) =
            q_node.get(scroll_area_id).unwrap();
        let scroll_area_affine: Affine2 = scroll_area_transform.into();
        let scroll_area_size =
            scroll_area_computed_node.size() * scroll_area_computed_node.inverse_scale_factor;
        let scroll_area_pos = scroll_area_affine.translation
            * scroll_area_computed_node.inverse_scale_factor
            - scroll_area_size * 0.5;

        // Get mutable access to the scroll position and content size info.
        let Ok(mut scroll_pos) = q_scroll_area.get_mut(scroll_area_id) else {
            return;
        };

        // Position of the target relative to the scroll area's top-left.
        let target_local_top_left = target_pos - scroll_area_pos + scroll_pos.0;
        let target_local_bottom_right = target_local_top_left + target_size;

        let content_size = scroll_area_computed_node.content_size()
            * scroll_area_computed_node.inverse_scale_factor;
        let max_range = (content_size - scroll_area_size).max(Vec2::ZERO);

        let can_scroll_x = scroll_area_node.overflow.x == OverflowAxis::Scroll;
        let can_scroll_y = scroll_area_node.overflow.y == OverflowAxis::Scroll;

        // Adjust by the minimal amount to make the target fully visible.
        if can_scroll_x {
            let view_min = scroll_pos.x;
            let view_max = scroll_pos.x + scroll_area_size.x;

            if target_local_top_left.x < view_min {
                scroll_pos.x = target_local_top_left.x.clamp(0.0, max_range.x);
            } else if target_local_bottom_right.x > view_max {
                scroll_pos.x =
                    (target_local_bottom_right.x - scroll_area_size.x).clamp(0.0, max_range.x);
            }
        }

        if can_scroll_y {
            let view_min = scroll_pos.y;
            let view_max = scroll_pos.y + scroll_area_size.y;

            if target_local_top_left.y < view_min {
                scroll_pos.y = target_local_top_left.y.clamp(0.0, max_range.y);
            } else if target_local_bottom_right.y > view_max {
                scroll_pos.y =
                    (target_local_bottom_right.y - scroll_area_size.y).clamp(0.0, max_range.y);
            }
        }
    }
}

/// Plugin that adds the observers for the [`ScrollArea`] widget.
pub struct ScrollAreaPlugin;

impl Plugin for ScrollAreaPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(scrollarea_on_scroll)
            .add_observer(on_scroll_into_view);
    }
}
