//! This module contains systems that update the UI when something changes

use crate::{CalculatedClip, Display, OverflowAxis, Style, TargetCamera, UiChildren, UiRootNodes};

use super::Node;
use bevy_ecs::{
    entity::Entity,
    query::{Changed, With},
    system::{Commands, Query},
};
use bevy_math::Rect;
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashSet;

/// Updates clipping for all nodes
pub fn update_clipping_system(
    mut commands: Commands,
    root_nodes: UiRootNodes,
    mut node_query: Query<(&Node, &GlobalTransform, &Style, Option<&mut CalculatedClip>)>,
    ui_children: UiChildren,
) {
    for root_node in root_nodes.iter() {
        update_clipping(
            &mut commands,
            &ui_children,
            &mut node_query,
            root_node,
            None,
        );
    }
}

fn update_clipping(
    commands: &mut Commands,
    ui_children: &UiChildren,
    node_query: &mut Query<(&Node, &GlobalTransform, &Style, Option<&mut CalculatedClip>)>,
    entity: Entity,
    mut maybe_inherited_clip: Option<Rect>,
) {
    let Ok((node, global_transform, style, maybe_calculated_clip)) = node_query.get_mut(entity)
    else {
        return;
    };

    // If `display` is None, clip the entire node and all its descendants by replacing the inherited clip with a default rect (which is empty)
    if style.display == Display::None {
        maybe_inherited_clip = Some(Rect::default());
    }

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
        commands.entity(entity).try_insert(CalculatedClip {
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
        let mut node_rect =
            Rect::from_center_size(global_transform.translation().truncate(), node.size());
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

    for child in ui_children.iter_ui_children(entity) {
        update_clipping(commands, ui_children, node_query, child, children_clip);
    }
}

pub fn update_target_camera_system(
    mut commands: Commands,
    changed_root_nodes_query: Query<
        (Entity, Option<&TargetCamera>),
        (With<Node>, Changed<TargetCamera>),
    >,
    node_query: Query<(Entity, Option<&TargetCamera>), With<Node>>,
    ui_root_nodes: UiRootNodes,
    ui_children: UiChildren,
) {
    // Track updated entities to prevent redundant updates, as `Commands` changes are deferred,
    // and updates done for changed_children_query can overlap with itself or with root_node_query
    let mut updated_entities = HashSet::new();

    // Assuming that TargetCamera is manually set on the root node only,
    // update root nodes first, since it implies the biggest change
    for (root_node, target_camera) in changed_root_nodes_query.iter_many(ui_root_nodes.iter()) {
        update_children_target_camera(
            root_node,
            target_camera,
            &node_query,
            &ui_children,
            &mut commands,
            &mut updated_entities,
        );
    }

    // If the root node TargetCamera was changed, then every child is updated
    // by this point, and iteration will be skipped.
    // Otherwise, update changed children
    for (parent, target_camera) in &node_query {
        if !ui_children.is_changed(parent) {
            continue;
        }

        update_children_target_camera(
            parent,
            target_camera,
            &node_query,
            &ui_children,
            &mut commands,
            &mut updated_entities,
        );
    }
}

fn update_children_target_camera(
    entity: Entity,
    camera_to_set: Option<&TargetCamera>,
    node_query: &Query<(Entity, Option<&TargetCamera>), With<Node>>,
    ui_children: &UiChildren,
    commands: &mut Commands,
    updated_entities: &mut HashSet<Entity>,
) {
    for child in ui_children.iter_ui_children(entity) {
        // Skip if the child has already been updated or update is not needed
        if updated_entities.contains(&child)
            || camera_to_set == node_query.get(child).ok().and_then(|(_, camera)| camera)
        {
            continue;
        }

        match camera_to_set {
            Some(camera) => {
                commands.entity(child).try_insert(camera.clone());
            }
            None => {
                commands.entity(child).remove::<TargetCamera>();
            }
        }
        updated_entities.insert(child);

        update_children_target_camera(
            child,
            camera_to_set,
            node_query,
            ui_children,
            commands,
            updated_entities,
        );
    }
}
