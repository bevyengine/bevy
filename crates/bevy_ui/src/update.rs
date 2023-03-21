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
    clip: Option<Rect>,
) {
    let (node, global_transform, style, old_clip) = node_query.get_mut(entity).unwrap();

    // Update current node's CalculatedClip component
    match (old_clip, clip) {
        (None, None) => {}
        (Some(_), None) => {
            commands.entity(entity).remove::<CalculatedClip>();
        }
        (None, Some(clip)) => {
            commands.entity(entity).insert(CalculatedClip { clip });
        }
        (Some(mut old_clip), Some(clip)) => {
            if old_clip.clip != clip {
                *old_clip = CalculatedClip { clip };
            }
        }
    }

    // Calculate new clip rectangle for children nodes
    let children_clip = match style.overflow {
        // When `Visible`, children will be visible even when they are outside
        // the current node's "area". In this case they inherit the current
        // node's parent clip. If an ancestor is set as `Hidden`, that clip will
        // be used; otherwise this will be `None`.
        Overflow::Visible => clip,
        Overflow::Hidden => {
            // Calculate current node clip rectangle from: posisition + calculated_size
            let node_center = global_transform.translation().truncate();
            let node_clip = Rect::from_center_size(node_center, node.calculated_size);

            // If `clip` is `Some`, use the intersection between current node's
            // clip and the ancestor `clip`. This handles the case of nested
            // `Overflow::Hidden` nodes. If parent `clip` is not defined, use
            // the current node's clip (pos + calculated size).
            Some(clip.map_or(node_clip, |c| c.intersect(node_clip)))
        }
    };

    if let Ok(children) = children_query.get(entity) {
        for &child in children.into_iter() {
            update_clipping(commands, children_query, node_query, child, children_clip);
        }
    }
}
