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
    let (node, global_transform, style, calculated_clip) = node_query.get_mut(entity).unwrap();
    // Update this node's CalculatedClip component
    match (clip, calculated_clip) {
        (None, None) => {}
        (None, Some(_)) => {
            commands.entity(entity).remove::<CalculatedClip>();
        }
        (Some(clip), None) => {
            commands.entity(entity).insert(CalculatedClip { clip });
        }
        (Some(clip), Some(mut old_clip)) => {
            if old_clip.clip != clip {
                *old_clip = CalculatedClip { clip };
            }
        }
    }

    // Calculate new clip for its children
    let children_clip = match style.overflow {
        Overflow::Visible => clip,
        Overflow::Hidden => {
            let node_center = global_transform.translation().truncate();
            let node_rect = Rect::from_center_size(node_center, node.calculated_size);
            Some(clip.map_or(node_rect, |c| c.intersect(node_rect)))
        }
    };

    if let Ok(children) = children_query.get(entity) {
        for child in children.iter().cloned() {
            update_clipping(commands, children_query, node_query, child, children_clip);
        }
    }
}
