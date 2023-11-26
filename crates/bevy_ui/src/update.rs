//! This module contains systems that update the UI when something changes

use crate::{CalculatedClip, OverflowAxis, Style};

use super::Node;
use bevy_ecs::{
    entity::Entity,
    query::{With, Without},
    system::{Commands, Query},
};
use bevy_hierarchy::{Children, Parent};
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
    let Ok((node, global_transform, style, maybe_calculated_clip)) = node_query.get_mut(entity)
    else {
        return;
    };

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
