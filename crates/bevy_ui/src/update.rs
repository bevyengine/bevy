//! This module contains systems that update the UI when something changes

use crate::{CalculatedClip, Interaction, OverflowAxis, ScrollPosition, Style};

use super::Node;
use bevy_ecs::{
    entity::Entity,
    event::EventReader,
    query::{Changed, With, Without},
    system::{Commands, Query},
};
use bevy_hierarchy::{Children, Parent};
use bevy_input::mouse::{MouseScrollUnit, MouseWheel};
use bevy_math::Rect;
use bevy_transform::components::GlobalTransform;

/// Updates clipping for all nodes
pub fn update_clipping_system(
    mut commands: Commands,
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    mut node_query: Query<(&Node, &GlobalTransform, &Style, Option<&mut CalculatedClip>)>,
    children_query: Query<&Children>,
) {
    for root_node in &root_node_query {
        update_clipping(
            &mut commands,
            &children_query,
            &mut node_query,
            root_node,
            None,
        );
    }
}

fn update_clipping(
    commands: &mut Commands,
    children_query: &Query<&Children>,
    node_query: &mut Query<(&Node, &GlobalTransform, &Style, Option<&mut CalculatedClip>)>,
    entity: Entity,
    maybe_inherited_clip: Option<Rect>,
) {
    let (node, global_transform, style, maybe_calculated_clip) =
        node_query.get_mut(entity).unwrap();

    // Update this node's CalculatedClip component
    if let Some(mut calculated_clip) = maybe_calculated_clip {
        if let Some(inherited_clip) = maybe_inherited_clip {
            // Replace the previous calculated clip with the inherited clipping rect
            if calculated_clip.clip != inherited_clip {
                *calculated_clip = CalculatedClip {
                    clip: inherited_clip,
                };
            }
        } else {
            // No inherited clipping rect, remove the component
            commands.entity(entity).remove::<CalculatedClip>();
        }
    } else if let Some(inherited_clip) = maybe_inherited_clip {
        // No previous calculated clip, add a new CalculatedClip component with the inherited clipping rect
        commands.entity(entity).insert(CalculatedClip {
            clip: inherited_clip,
        });
    }

    // Calculate new clip rectangle for children nodes
    let children_clip = if style.overflow.is_visible() {
        // When `Visible`, children might be visible even when they are outside
        // the current node's boundaries. In this case they inherit the current
        // node's parent clip. If an ancestor is set as `Hidden`, that clip will
        // be used; otherwise this will be `None`.
        maybe_inherited_clip
    } else {
        // If `maybe_inherited_clip` is `Some`, use the intersection between
        // current node's clip and the inherited clip. This handles the case
        // of nested `Overflow::Hidden` nodes. If parent `clip` is not
        // defined, use the current node's clip.
        let mut node_rect = node.logical_rect(global_transform);
        if style.overflow.x == OverflowAxis::Visible {
            node_rect.min.x = -f32::INFINITY;
            node_rect.max.x = f32::INFINITY;
        }
        if style.overflow.y == OverflowAxis::Visible {
            node_rect.min.y = -f32::INFINITY;
            node_rect.max.y = f32::INFINITY;
        }
        Some(maybe_inherited_clip.map_or(node_rect, |c| c.intersect(node_rect)))
    };

    if let Ok(children) = children_query.get(entity) {
        for &child in children {
            update_clipping(commands, children_query, node_query, child, children_clip);
        }
    }
}

pub fn update_scroll_interaction(
    mut interaction_query: Query<(&Interaction, &mut ScrollPosition), Changed<Interaction>>,
) {
    for (interaction, mut scroll) in &mut interaction_query {
        match *interaction {
            Interaction::Hovered | Interaction::Clicked => {
                scroll.is_hovered = true;
            }
            Interaction::None => {
                scroll.is_hovered = false;
            }
        }
    }
}

pub fn update_scroll_position(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&mut ScrollPosition, &Style, &Children, &Node)>,
    query_node: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.iter() {
        let (dx, dy) = match mouse_wheel_event.unit {
            MouseScrollUnit::Line => (mouse_wheel_event.x * 20., mouse_wheel_event.y * 20.),
            MouseScrollUnit::Pixel => (mouse_wheel_event.x, mouse_wheel_event.y),
        };

        for (mut scroll_container, style, children, list_node) in &mut query_list {
            if scroll_container.is_hovered {
                if style.overflow.x == OverflowAxis::Scroll {
                    let container_width = list_node.size().x;
                    let items_width: f32 = children
                        .iter()
                        .map(|child| query_node.get(*child).unwrap().size().x)
                        .sum();

                    let max_scroll_x = (items_width - container_width).max(0.);
                    scroll_container.offset_x =
                        (scroll_container.offset_x + dx).clamp(-max_scroll_x, 0.);
                }
                if style.overflow.y == OverflowAxis::Scroll {
                    let container_height = list_node.size().y;
                    let items_height: f32 = children
                        .iter()
                        .map(|child| query_node.get(*child).unwrap().size().y)
                        .sum();

                    let max_scroll_y = (items_height - container_height).max(0.);
                    scroll_container.offset_y =
                        (scroll_container.offset_y + dy).clamp(-max_scroll_y, 0.);
                }
            }
        }
    }
}
