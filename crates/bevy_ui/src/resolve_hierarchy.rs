use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::query::Changed;
use bevy_ecs::query::Has;
use bevy_ecs::query::With;
use bevy_ecs::query::Without;
use bevy_ecs::removal_detection::RemovedComponents;
use bevy_ecs::system::Commands;
use bevy_ecs::system::ParamSet;
use bevy_ecs::system::Query;
use bevy_ecs::world::Mut;
use bevy_ecs::world::Ref;

#[cfg(feature = "ghost_nodes")]
use crate::experimental::GhostNode;
use crate::experimental::UiChildren;
use crate::experimental::UiRootNodes;
use crate::Node;

#[derive(Component, PartialEq)]
pub struct ResolvedChildOf(pub(crate) Entity);

#[derive(Component, PartialEq)]
pub struct ResolvedChildren(pub(crate) Vec<Entity>);

#[cfg(feature = "ghost_nodes")]
pub fn mark_ghost_parent_nodes_changed(
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
        find_node_parent_of_ghost_recursively(parent, &mut param_set.p1());
    }
}

#[cfg(feature = "ghost_nodes")]
fn find_node_parent_of_ghost_recursively(
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
            find_node_parent_of_ghost_recursively(parent, node_query);
        }
        return;
    }
}

pub fn removed_children(
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

pub fn resolve_ui_hierarchy(
    mut commands: Commands,
    nodes_with_changed_children_query: Query<Entity, (Changed<Children>, With<Node>)>,
    mut resolved_children: Query<&mut ResolvedChildren>,
    mut resolved_childof: Query<&mut ResolvedChildOf>,
    ui_children: UiChildren,
) {
    for parent in nodes_with_changed_children_query.iter() {
        if let Ok(mut resolved_children) = resolved_children.get_mut(parent) {
            resolved_children.0.clear();
            resolved_children
                .0
                .extend(ui_children.iter_ui_children(parent));
        } else {
            commands.entity(parent).insert(ResolvedChildren(
                ui_children.iter_ui_children(parent).collect(),
            ));
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
