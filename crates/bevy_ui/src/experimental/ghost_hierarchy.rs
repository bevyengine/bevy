//! This module contains [`GhostNode`] and utilities to flatten the UI hierarchy, traversing past ghost nodes.

use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_hierarchy::{Children, HierarchyQueryExt, Parent};
use bevy_reflect::prelude::*;
use bevy_render::view::Visibility;
use bevy_transform::prelude::Transform;
use core::marker::PhantomData;
use smallvec::SmallVec;

use crate::Node;

/// Marker component for entities that should be ignored within UI hierarchies.
///
/// The UI systems will traverse past these and treat their first non-ghost descendants as direct children of their first non-ghost ancestor.
///
/// Any components necessary for transform and visibility propagation will be added automatically.
///
/// Instances of this type cannot be constructed unless the `ghost_nodes` feature is enabled.
#[derive(Component, Debug, Copy, Clone, Reflect)]
#[cfg_attr(feature = "ghost_nodes", derive(Default))]
#[reflect(Component, Debug)]
#[require(Visibility, Transform)]
pub struct GhostNode {
    // This is a workaround to ensure that GhostNode is only constructable when the appropriate feature flag is enabled
    #[reflect(ignore)]
    unconstructable: PhantomData<()>, // Spooky!
}

#[cfg(feature = "ghost_nodes")]
impl GhostNode {
    /// Creates a new ghost node.
    ///
    /// This method is only available when the `ghost_node` feature is enabled,
    /// and will eventually be deprecated then removed in favor of simply using `GhostNode` as no meaningful data is stored.
    pub const fn new() -> Self {
        GhostNode {
            unconstructable: PhantomData,
        }
    }
}

/// System param that allows iteration of all UI root nodes.
///
/// A UI root node is either a [`Node`] without a [`Parent`], or with only [`GhostNode`] ancestors.
#[derive(SystemParam)]
pub struct UiRootNodes<'w, 's> {
    root_node_query: Query<'w, 's, Entity, (With<Node>, Without<Parent>)>,
    root_ghost_node_query: Query<'w, 's, Entity, (With<GhostNode>, Without<Parent>)>,
    all_nodes_query: Query<'w, 's, Entity, With<Node>>,
    ui_children: UiChildren<'w, 's>,
}

impl<'w, 's> UiRootNodes<'w, 's> {
    pub fn iter(&'s self) -> impl Iterator<Item = Entity> + 's {
        self.root_node_query
            .iter()
            .chain(self.root_ghost_node_query.iter().flat_map(|root_ghost| {
                self.all_nodes_query
                    .iter_many(self.ui_children.iter_ui_children(root_ghost))
            }))
    }
}

/// System param that gives access to UI children utilities, skipping over [`GhostNode`].
#[derive(SystemParam)]
pub struct UiChildren<'w, 's> {
    ui_children_query: Query<
        'w,
        's,
        (Option<&'static Children>, Has<GhostNode>),
        Or<(With<Node>, With<GhostNode>)>,
    >,
    changed_children_query: Query<'w, 's, Entity, Changed<Children>>,
    children_query: Query<'w, 's, &'static Children>,
    ghost_nodes_query: Query<'w, 's, Entity, With<GhostNode>>,
    parents_query: Query<'w, 's, &'static Parent>,
}

impl<'w, 's> UiChildren<'w, 's> {
    /// Iterates the children of `entity`, skipping over [`GhostNode`].
    ///
    /// Traverses the hierarchy depth-first to ensure child order.
    ///
    /// # Performance
    ///
    /// This iterator allocates if the `entity` node has more than 8 children (including ghost nodes).
    pub fn iter_ui_children(&'s self, entity: Entity) -> UiChildrenIter<'w, 's> {
        UiChildrenIter {
            stack: self
                .ui_children_query
                .get(entity)
                .map_or(SmallVec::new(), |(children, _)| {
                    children.into_iter().flatten().rev().copied().collect()
                }),
            query: &self.ui_children_query,
        }
    }

    /// Returns the UI parent of the provided entity, skipping over [`GhostNode`].
    pub fn get_parent(&'s self, entity: Entity) -> Option<Entity> {
        self.parents_query
            .iter_ancestors(entity)
            .find(|entity| !self.ghost_nodes_query.contains(*entity))
    }

    /// Iterates the [`GhostNode`]s between this entity and its UI children.
    pub fn iter_ghost_nodes(&'s self, entity: Entity) -> Box<dyn Iterator<Item = Entity> + 's> {
        Box::new(
            self.children_query
                .get(entity)
                .into_iter()
                .flat_map(|children| {
                    self.ghost_nodes_query
                        .iter_many(children)
                        .flat_map(|entity| {
                            core::iter::once(entity).chain(self.iter_ghost_nodes(entity))
                        })
                }),
        )
    }

    /// Given an entity in the UI hierarchy, check if its set of children has changed, e.g if children has been added/removed or if the order has changed.
    pub fn is_changed(&'s self, entity: Entity) -> bool {
        self.changed_children_query.contains(entity)
            || self
                .iter_ghost_nodes(entity)
                .any(|entity| self.changed_children_query.contains(entity))
    }

    /// Returns `true` if the given entity is either a [`Node`] or a [`GhostNode`].
    pub fn is_ui_node(&'s self, entity: Entity) -> bool {
        self.ui_children_query.contains(entity)
    }
}

pub struct UiChildrenIter<'w, 's> {
    stack: SmallVec<[Entity; 8]>,
    query: &'s Query<
        'w,
        's,
        (Option<&'static Children>, Has<GhostNode>),
        Or<(With<Node>, With<GhostNode>)>,
    >,
}

impl<'w, 's> Iterator for UiChildrenIter<'w, 's> {
    type Item = Entity;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entity = self.stack.pop()?;
            if let Ok((children, has_ghost_node)) = self.query.get(entity) {
                if !has_ghost_node {
                    return Some(entity);
                }
                if let Some(children) = children {
                    self.stack.extend(children.iter().rev().copied());
                }
            }
        }
    }
}

#[cfg(all(test, feature = "ghost_nodes"))]
mod tests {
    use bevy_ecs::{
        prelude::Component,
        system::{Query, SystemState},
        world::World,
    };
    use bevy_hierarchy::{BuildChildren, ChildBuild};

    use super::{GhostNode, Node, UiChildren, UiRootNodes};

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[test]
    fn iterate_ui_root_nodes() {
        let world = &mut World::new();

        // Normal root
        world
            .spawn((A(1), Node::default()))
            .with_children(|parent| {
                parent.spawn((A(2), Node::default()));
                parent
                    .spawn((A(3), GhostNode::new()))
                    .with_child((A(4), Node::default()));
            });

        // Ghost root
        world
            .spawn((A(5), GhostNode::new()))
            .with_children(|parent| {
                parent.spawn((A(6), Node::default()));
                parent
                    .spawn((A(7), GhostNode::new()))
                    .with_child((A(8), Node::default()))
                    .with_child(A(9));
            });

        let mut system_state = SystemState::<(UiRootNodes, Query<&A>)>::new(world);
        let (ui_root_nodes, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(ui_root_nodes.iter()).collect();

        assert_eq!([&A(1), &A(6), &A(8)], result.as_slice());
    }

    #[test]
    fn iterate_ui_children() {
        let world = &mut World::new();

        let n1 = world.spawn((A(1), Node::default())).id();
        let n2 = world.spawn((A(2), GhostNode::new())).id();
        let n3 = world.spawn((A(3), GhostNode::new())).id();
        let n4 = world.spawn((A(4), Node::default())).id();
        let n5 = world.spawn((A(5), Node::default())).id();

        let n6 = world.spawn((A(6), GhostNode::new())).id();
        let n7 = world.spawn((A(7), GhostNode::new())).id();
        let n8 = world.spawn((A(8), Node::default())).id();
        let n9 = world.spawn((A(9), GhostNode::new())).id();
        let n10 = world.spawn((A(10), Node::default())).id();

        let no_ui = world.spawn_empty().id();

        world.entity_mut(n1).add_children(&[n2, n3, n4, n6]);
        world.entity_mut(n2).add_children(&[n5]);

        world.entity_mut(n6).add_children(&[n7, no_ui, n9]);
        world.entity_mut(n7).add_children(&[n8]);
        world.entity_mut(n9).add_children(&[n10]);

        let mut system_state = SystemState::<(UiChildren, Query<&A>)>::new(world);
        let (ui_children, a_query) = system_state.get(world);

        let result: Vec<_> = a_query
            .iter_many(ui_children.iter_ui_children(n1))
            .collect();

        assert_eq!([&A(5), &A(4), &A(8), &A(10)], result.as_slice());
    }
}
