//! This module contains systems that update the UI when something changes

use crate::{CalculatedClip, Overflow, Style};

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

    // Update current node's CalculatedClip component
    match (maybe_calculated_clip, maybe_inherited_clip) {
        (None, None) => {}
        (Some(_), None) => {
            commands.entity(entity).remove::<CalculatedClip>();
        }
        (None, Some(inherited_clip)) => {
            commands.entity(entity).insert(CalculatedClip {
                clip: inherited_clip,
            });
        }
        (Some(mut calculated_clip), Some(inherited_clip)) => {
            if calculated_clip.clip != inherited_clip {
                *calculated_clip = CalculatedClip {
                    clip: inherited_clip,
                };
            }
        }
    }

    // Calculate new clip rectangle for children nodes
    let children_clip = match style.overflow {
        // When `Visible`, children might be visible even when they are outside
        // the current node's boundaries. In this case they inherit the current
        // node's parent clip. If an ancestor is set as `Hidden`, that clip will
        // be used; otherwise this will be `None`.
        Overflow::Visible => maybe_inherited_clip,
        Overflow::Hidden => {
            let node_clip = node.logical_rect(global_transform);

            // If `maybe_inherited_clip` is `Some`, use the intersection between
            // current node's clip and the inherited clip. This handles the case
            // of nested `Overflow::Hidden` nodes. If parent `clip` is not
            // defined, use the current node's clip.
            Some(maybe_inherited_clip.map_or(node_clip, |c| c.intersect(node_clip)))
        }
    };

    if let Ok(children) = children_query.get(entity) {
        for &child in children {
            update_clipping(commands, children_query, node_query, child, children_clip);
        }
    }
}
