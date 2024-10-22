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
    ui_tree: UiTree<'w, 's>,
}

impl<'w, 's> UiRootNodes<'w, 's> {
    pub fn iter(&'s self) -> impl Iterator<Item = Entity> + 's {
        self.root_node_query
            .iter()
            .chain(self.root_ghost_node_query.iter().flat_map(|root_ghost| {
                self.all_nodes_query
                    .iter_many(self.ui_tree.iter_children(root_ghost))
            }))
    }
}

/// System param that gives access to UI tree utilities, skipping over [`GhostNode`] and stopping traversal at non-UI entities.
#[derive(SystemParam)]
pub struct UiTree<'w, 's> {
    ui_children_query: Query<
        'w,
        's,
        (Option<&'static Children>, Has<GhostNode>),
        Or<(With<Node>, With<GhostNode>)>,
    >,
    changed_children_query: Query<'w, 's, Entity, Changed<Children>>,
    ghost_nodes_query: Query<'w, 's, Entity, With<GhostNode>>,
    children_query: Query<'w, 's, &'static Children, Or<(With<Node>, With<GhostNode>)>>,
    parents_query: Query<'w, 's, &'static Parent, Or<(With<Node>, With<GhostNode>)>>,
}

impl<'w, 's> UiTree<'w, 's> {
    /// Iterates the [`Node`] children of `entity`, skipping over [`GhostNode`].
    ///
    /// Traverses the hierarchy depth-first to ensure child order.
    ///
    /// # Performance
    ///
    /// This iterator allocates if the `entity` node has more than 8 children (including ghost nodes).
    pub fn iter_children(&'s self, entity: Entity) -> UiChildrenIter<'w, 's> {
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

    /// Returns the [`Node`] parent of the provided entity, skipping over [`GhostNode`].
    pub fn parent(&'s self, entity: Entity) -> Option<Entity> {
        self.parents_query
            .iter_ancestors(entity)
            .find(|entity| !self.ghost_nodes_query.contains(*entity))
    }

    /// Returns the topmost [`Node`] ancestor of the given `entity`.
    ///
    /// This may be the entity itself if it has no parent or if it isn't part of a UI tree.
    pub fn root_ancestor(&'s self, entity: Entity) -> Entity {
        // Recursively search up the tree until we're out of parents
        match self.parent(entity) {
            Some(parent) => self.root_ancestor(parent),
            None => entity,
        }
    }

    /// Returns an [`Iterator`] of [`Node`] entities over all `entity`s ancestors within the current UI tree.
    ///
    /// Does not include the entity itself.
    pub fn iter_ancestors(&'s self, entity: Entity) -> impl Iterator<Item = Entity> + 's {
        self.parents_query
            .iter_ancestors(entity)
            .filter(|entity| !self.ghost_nodes_query.contains(*entity))
    }

    /// Returns an [`Iterator`] of [`Node`] entities over all of `entity`s descendants within the current UI tree.
    ///
    /// Traverses the hierarchy breadth-first and does not include the entity itself.
    pub fn iter_descendants(&'s self, entity: Entity) -> impl Iterator<Item = Entity> + 's {
        self.children_query
            .iter_descendants(entity)
            .filter(|entity| !self.ghost_nodes_query.contains(*entity))
    }

    /// Returns an [`Iterator`] of [`Node`] entities over all of `entity`s descendants within the current UI tree.
    ///
    /// Traverses the hierarchy depth-first and does not include the entity itself.
    pub fn iter_descendants_depth_first(
        &'s self,
        entity: Entity,
    ) -> impl Iterator<Item = Entity> + 's {
        self.children_query
            .iter_descendants_depth_first(entity)
            .filter(|entity| !self.ghost_nodes_query.contains(*entity))
    }

    /// Returns an [`Iterator`] of [`Node`] entities over the leaves of the UI tree underneath this `entity`.
    ///
    /// Only entities which have no [`Node`] descendants are considered leaves.
    /// This will not include the entity itself, and will not include any entities which are not descendants of the entity,
    /// even if they are leaves in the same hierarchical tree.
    ///
    /// Traverses the hierarchy depth-first.
    pub fn iter_leaves(&'s self, entity: Entity) -> impl Iterator<Item = Entity> + 's {
        UiLeavesIter {
            stack: self
                .ui_children_query
                .get(entity)
                .map_or(SmallVec::new(), |(children, _)| {
                    children.into_iter().flatten().rev().copied().collect()
                }),
            query: &self.ui_children_query,
            potential_leaf: None,
        }
    }

    /// Returns an [`Iterator`] of [`Node`] entities over the `entity`s immediate siblings, who share the same first [`Node`] ancestor within the UI tree.
    ///
    /// The entity itself is not included in the iterator.
    pub fn iter_siblings(&'s self, entity: Entity) -> impl Iterator<Item = Entity> + 's {
        self.parent(entity).into_iter().flat_map(move |parent| {
            self.iter_children(parent)
                .filter(move |child| *child != entity)
        })
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

    /// Given an entity in the UI tree, check if its parent has changed, e.g if children has been added/removed or if the order has changed.
    pub fn children_is_changed(&'s self, entity: Entity) -> bool {
        self.changed_children_query.contains(entity)
            || self
                .iter_ghost_nodes(entity)
                .any(|entity| self.changed_children_query.contains(entity))
    }

    /// Returns `true` if the given entity is either a [`Node`] or a [`GhostNode`].
    pub fn is_ui_entity(&'s self, entity: Entity) -> bool {
        self.ui_children_query.contains(entity)
    }

    /// Returns `true` if the given entity is a root in the UI tree.
    ///
    /// A [`Node`] is a root node if it has no parent, or if all ancestors are ghost nodes.
    ///
    /// A [`GhostNode`] is a root node if it has no parent.
    pub fn is_root(&'s self, entity: Entity) -> bool {
        self.parent(entity).is_none()
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

pub struct UiLeavesIter<'w, 's> {
    stack: SmallVec<[Entity; 8]>,
    query: &'s Query<
        'w,
        's,
        (Option<&'static Children>, Has<GhostNode>),
        Or<(With<Node>, With<GhostNode>)>,
    >,
    potential_leaf: Option<(usize, Entity)>,
}

impl<'w, 's> Iterator for UiLeavesIter<'w, 's> {
    type Item = Entity;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((stack_length, node_entity)) = self.potential_leaf {
                if stack_length == self.stack.len() {
                    // Confirmed leaf Node since no entities below it were Nodes
                    self.potential_leaf = None;
                    return Some(node_entity);
                }
            }

            let entity = self.stack.pop()?;
            if let Ok((children, has_ghost_node)) = self.query.get(entity) {
                if !has_ghost_node {
                    // This is a Node, store as potential leaf and continue traversing.
                    self.potential_leaf = Some((self.stack.len(), entity));
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

    use super::{GhostNode, Node, UiRootNodes, UiTree};

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
        let n11 = world.spawn((A(11), Node::default())).id();

        let no_ui = world.spawn_empty().id();

        world.entity_mut(n1).add_children(&[n2, n3, n4, n6]);
        world.entity_mut(n2).add_children(&[n5]);

        world.entity_mut(n6).add_children(&[n7, no_ui, n9]);
        world.entity_mut(n7).add_children(&[n8]);
        world.entity_mut(n9).add_children(&[n10]);
        world.entity_mut(n10).add_children(&[n11]);

        let mut system_state = SystemState::<(UiTree, Query<&A>)>::new(world);
        let (ui_tree, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(ui_tree.iter_children(n1)).collect();
        assert_eq!([&A(5), &A(4), &A(8), &A(10)], result.as_slice());

        let result: Vec<_> = a_query.iter_many(ui_tree.iter_descendants(n1)).collect();
        assert_eq!([&A(4), &A(5), &A(8), &A(10), &A(11)], result.as_slice());

        let result: Vec<_> = a_query
            .iter_many(ui_tree.iter_descendants_depth_first(n1))
            .collect();
        assert_eq!([&A(5), &A(4), &A(8), &A(10), &A(11)], result.as_slice());
    }

    #[test]
    fn ancestors() {
        let world = &mut World::new();

        let n1 = world.spawn((A(1), Node::default())).id();
        let n2 = world.spawn((A(2), GhostNode::new())).id();
        let n3 = world.spawn((A(3), GhostNode::new())).id();
        let n4 = world.spawn((A(4), Node::default())).id();
        let n5 = world.spawn((A(5), Node::default())).id();

        let n6 = world.spawn((A(6), GhostNode::new())).id();
        let n7 = world.spawn((A(7), GhostNode::new())).id();
        let n8 = world.spawn((A(8), Node::default())).id();

        world.entity_mut(n1).add_children(&[n2, n3]);
        world.entity_mut(n3).add_children(&[n4]);
        world.entity_mut(n4).add_children(&[n5]);

        world.entity_mut(n6).add_children(&[n7]);
        world.entity_mut(n7).add_children(&[n8]);

        let mut system_state = SystemState::<(UiTree, Query<&A>)>::new(world);
        let (ui_tree, a_query) = system_state.get(world);

        assert_eq!(&A(1), a_query.get(ui_tree.root_ancestor(n1)).unwrap());
        assert_eq!(&A(1), a_query.get(ui_tree.root_ancestor(n2)).unwrap());
        assert_eq!(&A(1), a_query.get(ui_tree.root_ancestor(n4)).unwrap());
        assert_eq!(&A(1), a_query.get(ui_tree.root_ancestor(n5)).unwrap());
        assert_eq!(&A(8), a_query.get(ui_tree.root_ancestor(n8)).unwrap());

        assert_eq!(
            [&A(1)],
            a_query
                .iter_many(ui_tree.iter_ancestors(n4))
                .collect::<Vec<_>>()
                .as_slice()
        );

        assert_eq!(
            [&A(4), &A(1)],
            a_query
                .iter_many(ui_tree.iter_ancestors(n5))
                .collect::<Vec<_>>()
                .as_slice()
        );

        assert!(ui_tree.iter_ancestors(n8).next().is_none());
    }

    #[test]
    fn iter_leaves() {
        let world = &mut World::new();

        let n1 = world.spawn((A(1), Node::default())).id();
        let n2 = world.spawn((A(2), GhostNode::new())).id();
        let n3 = world.spawn((A(3), Node::default())).id();
        let n4 = world.spawn((A(4), Node::default())).id();
        let n5 = world.spawn((A(5), Node::default())).id();
        let n6 = world.spawn((A(6), GhostNode::default())).id();

        world.entity_mut(n1).add_children(&[n2, n3]);
        world.entity_mut(n2).add_children(&[n4]);
        world.entity_mut(n3).add_children(&[n5]);
        world.entity_mut(n5).add_children(&[n6]);

        let mut system_state = SystemState::<(UiTree, Query<&A>)>::new(world);
        let (ui_tree, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(ui_tree.iter_leaves(n1)).collect();
        assert_eq!([&A(4), &A(5)], result.as_slice());
    }

    #[test]
    fn iter_siblings() {
        let world = &mut World::new();

        let n1 = world.spawn((A(1), Node::default())).id();
        let n2 = world.spawn((A(2), GhostNode::new())).id();
        let n3 = world.spawn((A(3), Node::default())).id();
        let n4 = world.spawn((A(4), Node::default())).id();
        let n5 = world.spawn((A(5), Node::default())).id();

        world.entity_mut(n1).add_children(&[n2, n3]);
        world.entity_mut(n2).add_children(&[n4, n5]);

        let mut system_state = SystemState::<(UiTree, Query<&A>)>::new(world);
        let (ui_tree, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(ui_tree.iter_siblings(n5)).collect();
        assert_eq!([&A(4), &A(3)], result.as_slice());
    }
}
