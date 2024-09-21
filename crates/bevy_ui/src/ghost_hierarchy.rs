//! This module contains [`GhostNode`] and utilities to flatten the UI hierarchy, traversing past ghost nodes.

use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_hierarchy::{Children, Parent};
use bevy_reflect::prelude::*;
use bevy_render::view::{InheritedVisibility, ViewVisibility, Visibility};
use bevy_transform::prelude::{GlobalTransform, Transform};
use smallvec::SmallVec;

use crate::Node;

/// Marker component for entities that should be ignored within UI hierarchies.
///
/// The UI systems will traverse past these and treat their first non-ghost descendants as direct children of their first non-ghost ancestor.
///
/// Any components necessary for transform and visibility propagation will be added automatically.
#[derive(Component, Default, Debug, Copy, Clone, Reflect)]
#[reflect(Component, Debug)]
#[require(
    Visibility,
    InheritedVisibility,
    ViewVisibility,
    Transform,
    GlobalTransform
)]
pub struct GhostNode;

/// System param that allows iteration of all UI root nodes.
///
/// A UI root node is either a [`Node`] without a [`Parent`], or with only [`GhostNode`] ancestors.
#[derive(SystemParam)]
pub struct UiRootNodes<'w, 's> {
    root_node_query: Query<'w, 's, Entity, (With<Node>, Without<Parent>)>,
    root_ghost_node_query: Query<'w, 's, Entity, (With<GhostNode>, Without<Parent>)>,
    ui_children: UiChildren<'w, 's>,
}

impl<'w, 's> UiRootNodes<'w, 's> {
    pub fn iter(&'s self) -> impl Iterator<Item = Entity> + 's {
        self.root_node_query.iter().chain(
            self.root_ghost_node_query
                .iter()
                .flat_map(|root_ghost| self.ui_children.iter(root_ghost)),
        )
    }
}

/// System param that allows iteration of UI children, skipping over [`GhostNode`].
///
/// Traverses the hierarchy depth-first to ensure child order.
#[derive(SystemParam)]
pub struct UiChildren<'w, 's> {
    query: Query<'w, 's, (Option<&'static Children>, Option<&'static GhostNode>)>,
}

impl<'w, 's> UiChildren<'w, 's> {
    pub fn iter(&'s self, entity: Entity) -> UiChildrenIter<'w, 's> {
        UiChildrenIter {
            stack: self
                .query
                .get(entity)
                .map_or(SmallVec::new(), |(children, _)| {
                    children.into_iter().flatten().rev().copied().collect()
                }),
            query: &self.query,
        }
    }
}

pub struct UiChildrenIter<'w, 's> {
    stack: SmallVec<[Entity; 8]>,
    query: &'s Query<'w, 's, (Option<&'static Children>, Option<&'static GhostNode>)>,
}

impl<'w, 's> Iterator for UiChildrenIter<'w, 's> {
    type Item = Entity;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entity = self.stack.pop()?;
            let (children, ghost_node) = self.query.get(entity).ok()?;
            if ghost_node.is_none() {
                return Some(entity);
            }
            if let Some(children) = children {
                self.stack.extend(children.iter().copied());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        prelude::Component,
        system::{Query, SystemState},
        world::World,
    };
    use bevy_hierarchy::{BuildChildren, ChildBuild};

    use super::{GhostNode, UiChildren, UiRootNodes};
    use crate::prelude::NodeBundle;

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[test]
    fn iterate_ui_root_nodes() {
        let world = &mut World::new();

        // Normal root
        world
            .spawn((A(1), NodeBundle::default()))
            .with_children(|parent| {
                parent.spawn((A(2), NodeBundle::default()));
                parent
                    .spawn((A(3), GhostNode))
                    .with_child((A(4), NodeBundle::default()));
            });

        // Ghost root
        world.spawn((A(5), GhostNode)).with_children(|parent| {
            parent.spawn((A(6), NodeBundle::default()));
            parent
                .spawn((A(7), GhostNode))
                .with_child((A(8), NodeBundle::default()));
        });

        let mut system_state = SystemState::<(UiRootNodes, Query<&A>)>::new(world);
        let (ui_root_nodes, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(ui_root_nodes.iter()).collect();

        assert_eq!([&A(1), &A(6), &A(8)], result.as_slice());
    }

    #[test]
    fn iterate_ui_children() {
        let world = &mut World::new();

        let n1 = world.spawn((A(1), NodeBundle::default())).id();
        let n2 = world.spawn((A(2), GhostNode)).id();
        let n3 = world.spawn((A(3), GhostNode)).id();
        let n4 = world.spawn((A(4), NodeBundle::default())).id();
        let n5 = world.spawn((A(5), NodeBundle::default())).id();

        world.entity_mut(n1).add_children(&[n2, n3, n4]);
        world.entity_mut(n2).add_children(&[n5]);

        let mut system_state = SystemState::<(UiChildren, Query<&A>)>::new(world);
        let (ui_children, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(ui_children.iter(n1)).collect();

        assert_eq!([&A(5), &A(4)], result.as_slice());
    }
}
