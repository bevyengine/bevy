use crate::experimental::UiChildren;
use crate::Node;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::query::Changed;
use bevy_ecs::query::With;
use bevy_ecs::query::Without;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;

use {
    crate::experimental::GhostNode,
    bevy_ecs::{query::Has, system::ParamSet, world::Mut},
};

/// Resolved parent UI node after flattening ghost nodes
/// Automatically inserted and updated by `resolve_ui_hierarchy`
#[derive(Component, PartialEq)]
pub struct ResolvedChildOf(pub(crate) Entity);

/// Resolved children of a UI node after flattening ghost nodes
/// Automatically inserted and updated by `resolve_ui_hierarchy`
#[derive(Component, PartialEq)]
pub struct ResolvedChildren(pub(crate) Vec<Entity>);

/// For each `GhostNode` entity with changed children,
/// find its first `Node` entity ancestor and mark its children changed.
pub fn mark_ghost_ancestor_nodes_changed(
    mut param_set: ParamSet<(
        Query<&ChildOf, (With<GhostNode>, Changed<Children>)>,
        Query<(Option<&ChildOf>, Mut<Children>, Has<Node>)>,
    )>,
) {
    let parents = param_set
        .p0()
        .iter()
        .map(|child_of| child_of.get())
        .collect::<Vec<Entity>>();
    for parent in parents.into_iter() {
        mark_changed_children_of_node_ancestor(parent, &mut param_set.p1());
    }
}

/// Walk up the tree until a `Node` entity is found and mark its children changed.
/// Does nothing if no `Node` ancestor found.
fn mark_changed_children_of_node_ancestor(
    entity: Entity,
    node_query: &mut Query<(Option<&ChildOf>, Mut<Children>, Has<Node>)>,
) {
    if let Ok((child_of, mut children, has_node)) = node_query.get_mut(entity) {
        if has_node {
            children.set_changed();
            return;
        }
        if let Some(child_of) = child_of {
            let parent = child_of.get();
            mark_changed_children_of_node_ancestor(parent, node_query);
        }
        return;
    }
}

/// If `Children` or `ChildOf` is removed, remove their resolved equivalent as well.
///
/// Possible optimisation might be that instead of removing the `ResolvedChildren` and `ResolvedChildOf`
/// components, we could add `With<Children>` and With<ChildOf>` filters to their respective queries.
pub fn synchronise_removed_hierarchy_components(
    mut commands: Commands,
    removed_child_of: Query<Entity, (With<ResolvedChildren>, Without<Children>)>,
    removed_children: Query<Entity, (With<ResolvedChildOf>, Without<ChildOf>)>,
) {
    for entity in removed_child_of.iter() {
        commands.entity(entity).remove::<ResolvedChildOf>();
    }

    for entity in removed_children.iter() {
        commands.entity(entity).remove::<ResolvedChildren>();
    }
}

/// Update the `ResolvedChildren` and `ResolvedChildOf` components for any `Node` entity with
/// changed `Children` by flattening any ghost nodes.
pub fn resolve_ui_hierarchy(
    mut commands: Commands,
    nodes_with_changed_children_query: Query<Entity, (Changed<Children>, With<Node>)>,
    mut resolved_children: Query<&mut ResolvedChildren>,
    mut resolved_childof: Query<&mut ResolvedChildOf>,
    ui_children: UiChildren,
) {
    for parent in nodes_with_changed_children_query.iter() {
        let child_iter = ui_children.iter_ui_children(parent);
        if let Ok(mut resolved_children) = resolved_children.get_mut(parent) {
            resolved_children.0.clear();
            resolved_children.0.extend(child_iter);
        } else {
            commands
                .entity(parent)
                .insert(ResolvedChildren(child_iter.collect()));
        }

        for child in ui_children.iter_ui_children(parent) {
            if let Ok(mut child_of) = resolved_childof.get_mut(child) {
                child_of.set_if_neq(ResolvedChildOf(parent));
            } else {
                commands.entity(child).insert(ResolvedChildOf(parent));
            }
        }
    }
}
