//! This module contains systems that update the UI when something changes

use crate::{CalculatedClip, Display, OverflowAxis, ScrollPosition, Style, TargetCamera};

use super::Node;
use bevy_ecs::{
    entity::Entity, event::EventReader, query::{Changed, With, Without}, system::{Commands, Query, Res}
};
use bevy_hierarchy::{Children, Parent};
use bevy_input::mouse::{MouseScrollUnit, MouseWheel};
use bevy_math::{Rect, Vec2};
use bevy_picking::focus::HoverMap;
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashSet;

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

pub fn update_target_camera_system(
    mut commands: Commands,
    changed_root_nodes_query: Query<
        (Entity, Option<&TargetCamera>),
        (With<Node>, Without<Parent>, Changed<TargetCamera>),
    >,
    changed_children_query: Query<(Entity, Option<&TargetCamera>), (With<Node>, Changed<Children>)>,
    children_query: Query<&Children, With<Node>>,
    node_query: Query<Option<&TargetCamera>, With<Node>>,
) {
    // Track updated entities to prevent redundant updates, as `Commands` changes are deferred,
    // and updates done for changed_children_query can overlap with itself or with root_node_query
    let mut updated_entities = HashSet::new();

    // Assuming that TargetCamera is manually set on the root node only,
    // update root nodes first, since it implies the biggest change
    for (root_node, target_camera) in &changed_root_nodes_query {
        update_children_target_camera(
            root_node,
            target_camera,
            &node_query,
            &children_query,
            &mut commands,
            &mut updated_entities,
        );
    }

    // If the root node TargetCamera was changed, then every child is updated
    // by this point, and iteration will be skipped.
    // Otherwise, update changed children
    for (parent, target_camera) in &changed_children_query {
        update_children_target_camera(
            parent,
            target_camera,
            &node_query,
            &children_query,
            &mut commands,
            &mut updated_entities,
        );
    }
}

fn update_children_target_camera(
    entity: Entity,
    camera_to_set: Option<&TargetCamera>,
    node_query: &Query<Option<&TargetCamera>, With<Node>>,
    children_query: &Query<&Children, With<Node>>,
    commands: &mut Commands,
    updated_entities: &mut HashSet<Entity>,
) {
    let Ok(children) = children_query.get(entity) else {
        return;
    };

    for &child in children {
        // Skip if the child has already been updated or update is not needed
        if updated_entities.contains(&child)
            || camera_to_set == node_query.get(child).ok().flatten()
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
            children_query,
            commands,
            updated_entities,
        );
    }
}

pub fn update_scroll_position(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut scrolled_node_query: Query<(&mut ScrollPosition, &Style, &Children, &Node)>,
    just_node_query: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        // TODO: 90% sure this should be user-configurable, bevy shouldn't own scroll speed
        let (dx, dy) = match mouse_wheel_event.unit {
            MouseScrollUnit::Line => (mouse_wheel_event.x * 20., mouse_wheel_event.y * 20.),
            MouseScrollUnit::Pixel => (mouse_wheel_event.x, mouse_wheel_event.y),
        };

        for (_pointer, pointer_map) in hover_map.iter() {
            for (entity, _hit) in pointer_map.iter() {
                if let Ok((mut scroll_position, style, children, scrolled_node)) = scrolled_node_query.get_mut(*entity) {
                    let Vec2 {
                        x: container_width,
                        y: container_height,
                    } = scrolled_node.size();

                    let (items_width, items_height): (f32, f32) =
                        children.iter().fold((0.0, 0.0), |sum, child| {
                            let size = just_node_query.get(*child).unwrap().size();
                            (sum.0 + size.x, sum.1 + size.y)
                        });
    
                    if style.overflow.x == OverflowAxis::Scroll {
                        let max_scroll_x = (items_width - container_width).max(0.);
                        scroll_position.offset_x =
                            (scroll_position.offset_x + dx).clamp(-max_scroll_x, 0.);
                    }
                    if style.overflow.y == OverflowAxis::Scroll {
                        let max_scroll_y = (items_height - container_height).max(0.);
                        scroll_position.offset_y =
                            (scroll_position.offset_y + dy).clamp(-max_scroll_y, 0.);
                    }

                    println!("new scroll possy: {:?} | scroll node size {:?} | items_w {:?} items_h {:?}", scroll_position, scrolled_node.size(), items_width, items_height);
                }
            }
        }
    }
}
