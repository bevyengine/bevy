use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    observer::On,
    query::With,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_input::mouse::MouseScrollPixelsPerLine;
use bevy_math::Vec2;
use bevy_picking::events::PointerScroll;
use bevy_reflect::Reflect;
use bevy_ui::{ComputedNode, Node, OverflowAxis, ScrollPosition};

/// Marker component to enable trackpad / mouse wheel scrolling. This should be placed on an
/// entity that has overflow: scroll.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[require(ScrollPosition)]
#[reflect(Component)]
pub struct ScrollArea;

fn scrollarea_on_scroll(
    mut scroll: On<PointerScroll>,
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

/// Plugin that adds the observers for the [`ScrollArea`] widget.
pub struct ScrollAreaPlugin;

impl Plugin for ScrollAreaPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(scrollarea_on_scroll);
    }
}
