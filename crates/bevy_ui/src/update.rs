//! This module contains systems that update the UI when something changes

use crate::{CalculatedClip, Overflow, OverflowAxis, Style};

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
    let (node, global_transform, style, maybe_calculated_clip) =
        node_query.get_mut(entity).unwrap();

    // Update this node's CalculatedClip component
    match (maybe_inherited_clip, maybe_calculated_clip) {
        (None, Some(_)) => {
            commands.entity(entity).remove::<CalculatedClip>();
        }
        (Some(inherited_clip), None) => {
            commands.entity(entity).insert(CalculatedClip {
                clip: inherited_clip,
            });
        }
        (Some(inherited_clip), Some(mut calculated_clip)) => {
            *calculated_clip = CalculatedClip {
                clip: inherited_clip,
            }
        }
        _ => {}
    }

    // Calculate new clip for its children
    let children_clip = match style.overflow {
        Overflow {
            x: OverflowAxis::Visible,
            y: OverflowAxis::Visible,
        } => maybe_inherited_clip,
        _ => {
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
        }
    };

    if let Ok(children) = children_query.get(entity) {
        for &child in children {
            update_clipping(commands, children_query, node_query, child, children_clip);
        }
    }
}
