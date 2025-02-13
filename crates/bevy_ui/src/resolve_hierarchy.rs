use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;

use crate::experimental::UiChildren;
use crate::experimental::UiRootNodes;

#[derive(Component, PartialEq)]
pub struct ResolvedChildOf(pub(crate) Entity);

#[derive(Component, PartialEq)]
pub struct ResolvedChildren(pub(crate) Vec<Entity>);

pub fn resolve_ui_hierarchy(
    mut commands: Commands,
    ui_roots: UiRootNodes,
    ui_children: UiChildren,
    mut resolved_childof_query: Query<&mut ResolvedChildOf>,
    mut resolved_children_query: Query<&mut ResolvedChildren>,
) {
    for entity in ui_roots.iter() {
        resolve_ui_hierarchy_recursively(
            entity,
            &mut commands,
            &ui_children,
            &mut resolved_childof_query,
            &mut resolved_children_query,
        );
    }
}

pub fn resolve_ui_hierarchy_recursively(
    entity: Entity,
    commands: &mut Commands,
    ui_children: &UiChildren,
    resolved_childof_query: &mut Query<&mut ResolvedChildOf>,
    resolved_children_query: &mut Query<&mut ResolvedChildren>,
) {
    let children = ui_children
        .iter_ui_children(entity)
        .collect::<Vec<Entity>>();

    for &child in children.iter() {
        if let Ok(mut resolved_childof) = resolved_childof_query.get_mut(child) {
            resolved_childof.set_if_neq(ResolvedChildOf(entity));
        } else {
            commands.entity(child).insert(ResolvedChildOf(entity));
        }

        resolve_ui_hierarchy_recursively(
            child,
            commands,
            ui_children,
            resolved_childof_query,
            resolved_children_query,
        );
    }

    if let Ok(mut resolved_children) = resolved_children_query.get_mut(entity) {
        resolved_children.set_if_neq(ResolvedChildren(children));
    } else {
        commands.entity(entity).insert(ResolvedChildren(children));
    }
}
