//! This module contains systems that update the UI camera targets

use crate::{
    experimental::{UiChildren, UiRootNodes},
    Node, TargetCamera,
};

use bevy_ecs::{
    entity::Entity,
    query::{Changed, With},
    system::{Commands, Query},
};
use bevy_utils::HashSet;

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
